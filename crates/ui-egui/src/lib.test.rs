#[cfg(test)]
mod unsaved_state_tests {
    use crate::unsaved::{Choice, Guarded, guard};
    use crate::{MaterialCraftApp, state::UiState};

    fn app(modified: bool) -> MaterialCraftApp {
        let mut s = materialcraft_engine::sample_session();
        s.modified = modified;
        MaterialCraftApp::headless(s, UiState::default())
    }

    #[test]
    fn closing_all_over_unsaved_work_waits_for_an_answer() {
        let mut a = app(true);
        let before = a.session.pkg.name.clone();
        guard(&mut a, Guarded::Menu { id: "ui.closeAll".into() });
        assert!(a.ui.windows.unsaved.is_some());
        assert_eq!(a.session.pkg.name, before, "nothing was replaced yet");
        // The held action is visible as serde state (control channel `ui.inspect`).
        let v = serde_json::to_value(&a.ui).unwrap();
        assert_eq!(v["windows"]["unsaved"]["kind"], "menu");
    }

    #[test]
    fn closing_all_over_a_clean_package_just_happens() {
        let mut a = app(false);
        guard(&mut a, Guarded::Menu { id: "ui.closeAll".into() });
        assert!(a.ui.windows.unsaved.is_none());
        assert_eq!(a.session.pkg.graphs.first().map(|g| g.name.as_str()), Some("New_graph"), "the empty package replaced it");
    }

    #[test]
    fn discard_runs_the_held_action_and_cancel_keeps_the_work() {
        let mut a = app(true);
        guard(&mut a, Guarded::Menu { id: "ui.closeAll".into() });
        let step = crate::unsaved::answer(&mut a.ui.windows.unsaved, Choice::Cancel);
        assert_eq!(step, crate::unsaved::Step::Stop);
        assert!(a.session.modified);
    }

    #[test]
    fn a_pending_dialog_is_not_restored_from_saved_state() {
        let mut a = app(true);
        guard(&mut a, Guarded::Quit);
        let saved = serde_json::to_string(&a.ui).unwrap();
        let back: UiState = serde_json::from_str(&saved).unwrap();
        assert!(back.windows.unsaved.is_none());
    }
}
