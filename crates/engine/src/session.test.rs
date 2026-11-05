#[cfg(test)]
mod group_tests {
    use crate::{EngineError, Session, execute};
    use serde_json::json;

    fn two_nodes() -> (Session, u64, u64) {
        let mut s = crate::sample_session();
        execute(&mut s, "file.new", &json!({"template": "empty"})).unwrap();
        let a = execute(&mut s, "graph.node.add", &json!({"def": "clouds_1"})).unwrap()["id"].as_u64().unwrap();
        let b = execute(&mut s, "graph.node.add", &json!({"def": "levels"})).unwrap()["id"].as_u64().unwrap();
        (s, a, b)
    }

    #[test]
    fn a_group_is_one_undo_entry_and_one_undo_reverts_all() {
        let (mut s, a, b) = two_nodes();
        let base = s.history().len();
        s.group("Rewire", |s| {
            execute(s, "graph.connect", &json!({"from": a, "to": b, "toPort": "input"}))?;
            execute(s, "graph.disconnect", &json!({"to": b, "toPort": "input"}))?;
            execute(s, "graph.connect", &json!({"from": a, "to": b, "toPort": "input"}))?;
            Ok(())
        })
        .unwrap();
        assert_eq!(s.history().len(), base + 1);
        assert_eq!(s.history().last().map(String::as_str), Some("Rewire"));
        assert_eq!(s.undo().as_deref(), Some("Rewire"));
        assert!(s.graph().unwrap().connections.is_empty(), "the single undo removed every step");
    }

    #[test]
    fn a_failing_group_restores_everything_and_records_nothing() {
        let (mut s, a, b) = two_nodes();
        let base = s.history().len();
        let r: Result<(), EngineError> = s.group("Rewire", |s| {
            execute(s, "graph.connect", &json!({"from": a, "to": b, "toPort": "input"}))?;
            Err(EngineError::Other("boom".into()))
        });
        assert!(r.is_err());
        assert_eq!(s.history().len(), base);
        assert!(s.graph().unwrap().connections.is_empty());
    }

    #[test]
    fn begin_and_end_group_span_several_calls_and_an_untouched_group_records_nothing() {
        let (mut s, a, b) = two_nodes();
        let base = s.history().len();
        s.begin_group("Nothing");
        s.end_group();
        assert_eq!(s.history().len(), base);
        s.begin_group("Both");
        execute(&mut s, "graph.connect", &json!({"from": a, "to": b, "toPort": "input"})).unwrap();
        execute(&mut s, "graph.node.param", &json!({"id": b, "param": "in_low", "value": 0.2})).unwrap();
        s.end_group();
        assert_eq!(s.history().len(), base + 1);
    }
}
