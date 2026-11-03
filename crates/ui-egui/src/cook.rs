//! Background cooking: when the open graph changes, a worker thread cooks a snapshot of the
//! package through the shared cache while the UI keeps drawing the previous results.

use std::sync::mpsc::{Receiver, TryRecvError};

use materialcraft_engine::eval::Cooked;

use crate::MaterialCraftApp;

#[derive(Default)]
pub struct Cooker {
    rx: Option<Receiver<Cooked>>,
    /// Stops the running cook between nodes once its result is stale.
    #[cfg(not(target_arch = "wasm32"))]
    cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
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
            Ok(c) => {
                if c.cancelled {
                    // A stopped cook is partial: keep the previous results and cook again.
                    app.session.dirty = true;
                } else {
                    app.session.cooked = c;
                }
                app.cooker.rx = None;
                app.cooker.started = None;
                ctx.request_repaint();
            }
            Err(TryRecvError::Empty) if app.session.dirty => {
                // The running cook is stale: stop it, ignore its result and cook the new state.
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(flag) = &app.cooker.cancel {
                    flag.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                app.cooker.rx = None;
                app.cooker.started = None;
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
                return;
            }
            Err(TryRecvError::Disconnected) => {
                app.cooker.rx = None;
                app.cooker.started = None;
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
    let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let thread_flag = flag.clone();
    use materialcraft_engine::eval::Options;
    let spawned = std::thread::Builder::new().name("materialcraft-cook".into()).spawn(move || {
        let mut opts = Options::new(&dec);
        opts.max_size = max;
        opts.cancel = Some(&thread_flag);
        let cooked = materialcraft_engine::eval::cook(&pkg, gi, &opts, &cache);
        let _ = tx.send(cooked);
    });
    match spawned {
        Ok(_) => {
            app.cooker.rx = Some(rx);
            app.cooker.cancel = Some(flag);
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
