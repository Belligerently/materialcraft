#[cfg(test)]
mod unsaved_tests {
    use crate::unsaved::{Choice, Guarded, Step, after_save, answer, needs_confirmation, request};

    fn open() -> Guarded {
        Guarded::OpenPath { path: "a.sbs".into() }
    }

    #[test]
    fn a_clean_session_never_asks() {
        let mut s = materialcraft_engine::sample_session();
        s.modified = false;
        assert!(!needs_confirmation(&s));
        let mut pending = None;
        assert_eq!(request(&mut pending, false, open()), Some(open()));
        assert_eq!(pending, None);
    }

    #[test]
    fn a_modified_session_asks_and_holds_the_action() {
        let mut s = materialcraft_engine::sample_session();
        s.modified = true;
        assert!(needs_confirmation(&s));
        let mut pending = None;
        assert_eq!(request(&mut pending, true, open()), None);
        assert_eq!(pending, Some(open()));
    }

    #[test]
    fn the_first_pending_action_is_kept() {
        let mut pending = None;
        request(&mut pending, true, open());
        assert_eq!(request(&mut pending, true, Guarded::Quit), None);
        assert_eq!(pending, Some(open()));
    }

    #[test]
    fn cancel_drops_discard_runs_save_saves_first() {
        let mut pending = Some(open());
        assert_eq!(answer(&mut pending, Choice::Cancel), Step::Stop);
        assert_eq!(pending, None);

        let mut pending = Some(open());
        assert_eq!(answer(&mut pending, Choice::Discard), Step::Run(open()));
        assert_eq!(pending, None);

        let mut pending = Some(open());
        assert_eq!(answer(&mut pending, Choice::Save), Step::Save(open()));
        assert_eq!(pending, None);

        assert_eq!(answer(&mut None, Choice::Discard), Step::Stop, "nothing pending, nothing to run");
    }

    #[test]
    fn closing_the_window_is_held_only_for_unsaved_work() {
        use crate::unsaved::holds_close;
        assert!(holds_close(true, false));
        assert!(!holds_close(false, false));
        assert!(!holds_close(true, true), "after Discard the close goes through");
    }

    #[test]
    fn the_action_runs_after_save_only_when_the_save_worked() {
        assert_eq!(after_save(false, open()), Some(open()));
        assert_eq!(after_save(true, open()), None, "still modified: the save was cancelled or failed");
    }
}

#[test]
fn a_second_package_cannot_replace_the_payload_waiting_for_confirmation() {
    use crate::unsaved::{Choice, Guarded, Step};
    let mut app = crate::MaterialCraftApp::headless(materialcraft_engine::sample_session(), crate::UiState::default());
    app.session.modified = true;
    let bytes = |name: &str| materialcraft_engine::formats::sbs::write_sbs(&materialcraft_engine::core::Package::new(name)).unwrap();
    crate::web_files::load(&mut app, "first.sbs", bytes("First"), false);
    crate::web_files::load(&mut app, "second.sbs", bytes("Second"), false);
    assert_eq!(app.ui.windows.unsaved, Some(Guarded::LoadBytes { name: "first.sbs".into() }));
    let result = crate::control::handle(&mut app, &egui::Context::default(), "ui.set", &serde_json::json!({"prefs": {"thumbnails": false}}));
    assert_eq!(result["ok"], true);
    assert!(app.ui.windows.unsaved.is_some());
    if let Step::Run(action) = crate::unsaved::answer(&mut app.ui.windows.unsaved, Choice::Discard) {
        crate::unsaved::perform(&mut app, action);
    } else {
        panic!("missing held action");
    }
    assert_eq!(app.session.pkg.name, "First");
}

#[test]
fn a_pending_action_is_kept_even_if_the_package_was_saved_elsewhere() {
    let first = crate::unsaved::Guarded::Quit;
    let mut pending = Some(first.clone());
    assert!(crate::unsaved::request(&mut pending, false, crate::unsaved::Guarded::Menu { id: "ui.closeAll".into() }).is_none());
    assert_eq!(pending, Some(first));
}
