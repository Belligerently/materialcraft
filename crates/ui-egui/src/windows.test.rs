#[cfg(test)]
mod open_guard_tests {
    use crate::unsaved::Guarded;
    use crate::{MaterialCraftApp, state::UiState};

    #[test]
    fn opening_a_path_over_unsaved_work_asks_first() {
        let mut s = materialcraft_engine::sample_session();
        s.modified = true;
        let mut app = MaterialCraftApp::headless(s, UiState::default());
        crate::windows::open_path(&mut app, "does-not-exist.sbs");
        assert_eq!(app.ui.windows.unsaved, Some(Guarded::OpenPath { path: "does-not-exist.sbs".into() }));
        assert!(app.toasts.is_empty(), "nothing was attempted yet");
    }

    #[test]
    fn opening_a_path_over_a_clean_package_goes_straight_to_open() {
        let mut s = materialcraft_engine::sample_session();
        s.modified = false;
        let mut app = MaterialCraftApp::headless(s, UiState::default());
        crate::windows::open_path(&mut app, "does-not-exist.sbs");
        assert!(app.ui.windows.unsaved.is_none());
        assert!(!app.toasts.is_empty(), "the open was attempted and its error reported");
    }
}
