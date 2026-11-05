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
