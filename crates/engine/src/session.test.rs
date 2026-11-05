#[cfg(test)]
mod merge_tests {
    use crate::{EngineError, Session};

    fn session() -> Session {
        crate::sample_session()
    }

    fn set(s: &mut Session, key: &str) {
        s.edit_merged(key, "Set parameter", |s| {
            s.modified = true;
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn same_key_merges_until_end_merge() {
        let mut s = session();
        let base = s.history().len();
        set(&mut s, "k");
        set(&mut s, "k");
        assert_eq!(s.history().len(), base + 1, "one drag is one entry");
        s.end_merge();
        set(&mut s, "k");
        assert_eq!(s.history().len(), base + 2, "a second drag after end_merge is a new entry");
    }

    #[test]
    fn failed_merged_edit_restores_the_session() {
        let mut s = session();
        set(&mut s, "k");
        let name = s.pkg.name.clone();
        let r: Result<(), EngineError> = s.edit_merged("k", "Set parameter", |s| {
            s.pkg.name = "half-applied".into();
            Err(EngineError::Other("boom".into()))
        });
        assert!(r.is_err());
        assert_eq!(s.pkg.name, name);
    }
}
