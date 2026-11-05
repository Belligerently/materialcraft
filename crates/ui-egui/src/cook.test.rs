#[cfg(test)]
mod cook_tests {
    use crate::MaterialCraftApp;
    use crate::cook::{drive, guard};
    use crate::state::UiState;
    use materialcraft_engine::eval::Cooked;
    use materialcraft_engine::sample_session;

    fn app() -> MaterialCraftApp {
        let mut session = sample_session();
        session.preview_size = Some(64);
        session.cook();
        MaterialCraftApp::headless(session, UiState::default())
    }

    #[test]
    fn guard_turns_a_panic_into_an_error() {
        let r: Result<u32, String> = guard("test", || -> Result<u32, String> { panic!("boom") });
        let e = r.unwrap_err();
        assert!(e.contains("boom"), "{e}");
        assert_eq!(guard("test", || Ok::<_, String>(7)).unwrap(), 7);
    }

    #[test]
    fn a_panicked_cook_worker_reports_errors_on_every_node() {
        let mut app = app();
        assert!(app.session.cook_failures().0.is_empty());
        let (tx, rx) = std::sync::mpsc::channel::<Result<Cooked, String>>();
        app.cooker.rx = Some(rx);
        tx.send(Err("boom".into())).unwrap();
        app.session.dirty = false;
        drive(&mut app, &egui::Context::default());
        assert!(!app.cooker.busy());
        let (failed, _) = app.session.cook_failures();
        assert!(!failed.is_empty());
        assert!(failed.iter().all(|(_, e)| e.contains("boom")));
        assert!(app.toasts.iter().any(|t| t.error && t.text.contains("boom")));
    }

    #[test]
    fn a_dead_cook_worker_is_reported_not_silent() {
        let mut app = app();
        let (tx, rx) = std::sync::mpsc::channel::<Result<Cooked, String>>();
        app.cooker.rx = Some(rx);
        drop(tx);
        app.session.dirty = false;
        drive(&mut app, &egui::Context::default());
        assert!(!app.cooker.busy());
        assert!(app.toasts.iter().any(|t| t.error));
        assert!(!app.session.cook_failures().0.is_empty());
    }
}
