//! Background cooking: when the open graph changes, a worker thread cooks a snapshot of the
//! package through the shared cache while the UI keeps drawing the previous results.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{Receiver, TryRecvError};

use materialcraft_engine::eval::{Cooked, NodeResult};

use crate::MaterialCraftApp;

/// Run `f`, turning a panic into an error naming `what`, so one bad node or command can't take
/// the app (and unsaved work) down.
pub fn guard<T>(what: &str, f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|p| {
        let msg = p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_default();
        Err(format!("internal error in {what}: {msg}"))
    })
}

/// Run an engine command; a panic inside it becomes an error reply (the session is kept).
pub fn execute_guarded(session: &mut materialcraft_engine::Session, id: &str, params: &serde_json::Value) -> Result<serde_json::Value, String> {
    guard(&format!("command `{id}`"), || materialcraft_engine::execute(session, id, params).map_err(|e| e.to_string()))
}

/// A cook that crashed or died: every node of the open graph shows the error instead of stale
/// previews, and the user sees a toast.
fn report_failure(app: &mut MaterialCraftApp, msg: String) {
    let ids: Vec<_> = app.session.graph().map(|g| g.nodes.iter().map(|n| n.id).collect()).unwrap_or_default();
    app.session.cooked = Cooked::default();
    for id in ids {
        app.session.cooked.nodes.insert(id, NodeResult { outputs: Default::default(), error: Some(msg.clone()), ms: 0.0, key: 0, cached: false });
    }
    app.toast(msg, true);
}

#[derive(Default)]
pub struct Cooker {
    rx: Option<Receiver<Result<Cooked, String>>>,
    /// When the running cook started (app seconds).
    pub started: Option<f64>,
}

impl Cooker {
    pub fn busy(&self) -> bool {
        self.rx.is_some()
    }
}

/// Collect a finished cook and start the next one if the graph changed.
pub fn drive(app: &mut MaterialCraftApp, ctx: &egui::Context) {
    if let Some(rx) = &app.cooker.rx {
        match rx.try_recv() {
            Ok(result) => {
                app.cooker.rx = None;
                app.cooker.started = None;
                match result {
                    Ok(c) => app.session.cooked = c,
                    Err(msg) => report_failure(app, msg),
                }
                ctx.request_repaint();
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
                return;
            }
            Err(TryRecvError::Disconnected) => {
                app.cooker.rx = None;
                app.cooker.started = None;
                report_failure(app, "the cook worker stopped unexpectedly".into());
            }
        }
    }
    if app.session.dirty {
        start(app, ctx.input(|i| i.time));
        ctx.request_repaint();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn start(app: &mut MaterialCraftApp, now: f64) {
    let s = &mut app.session;
    s.dirty = false;
    let pkg = s.pkg.clone();
    let (gi, max) = (s.graph, s.preview_size);
    let cache = s.cache.clone();
    let dec = s.resource_decoder();
    let (tx, rx) = std::sync::mpsc::channel();
    use materialcraft_engine::eval::Options;
    use std::sync::PoisonError;
    let spawned = std::thread::Builder::new().name("materialcraft-cook".into()).spawn(move || {
        let mut opts = Options::new(&dec);
        opts.max_size = max;
        let cooked = guard("cook", || {
            let mut cache = cache.lock().unwrap_or_else(PoisonError::into_inner);
            Ok(materialcraft_engine::eval::cook(&pkg, gi, &opts, &mut cache))
        });
        let _ = tx.send(cooked);
    });
    match spawned {
        Ok(_) => {
            app.cooker.rx = Some(rx);
            app.cooker.started = Some(now);
        }
        Err(e) => {
            log::warn!("could not start a cook thread ({e}); cooking here");
            app.session.cook();
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn start(app: &mut MaterialCraftApp, _now: f64) {
    // shortcut: no threads on the web; the cook runs on the UI thread (the page pauses).
    app.session.cook();
}

#[cfg(test)]
#[path = "cook.test.rs"]
mod tests;
