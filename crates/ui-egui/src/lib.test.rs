#[cfg(test)]
mod gesture_tests {
    use crate::end_gesture_if_released;

    #[test]
    fn pointer_release_ends_the_merge_run() {
        let mut s = materialcraft_engine::sample_session();
        let base = s.history().len();
        let set = |s: &mut materialcraft_engine::Session| {
            s.edit_merged("k", "Set parameter", |_| Ok(())).unwrap();
        };
        set(&mut s);
        end_gesture_if_released(&mut s, false);
        set(&mut s);
        assert_eq!(s.history().len(), base + 1, "still dragging: merged");
        end_gesture_if_released(&mut s, true);
        set(&mut s);
        assert_eq!(s.history().len(), base + 2, "released: a new step");
    }
}
