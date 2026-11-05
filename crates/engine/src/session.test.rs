#[cfg(test)]
mod session_tests {
    use crate::Session;
    use materialcraft_core::Package;

    #[test]
    fn sync_cook_bumps_the_cook_epoch() {
        let mut s = Session::new(Package::new("t"));
        let before = s.cook_epoch;
        s.cook();
        assert_ne!(s.cook_epoch, before);
    }
}
