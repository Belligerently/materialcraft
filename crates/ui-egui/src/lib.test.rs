#[cfg(test)]
mod lib_tests {
    use crate::MaterialCraftApp;
    use crate::cook::execute_guarded;
    use crate::state::UiState;
    use materialcraft_engine::sample_session;
    use serde_json::json;

    #[test]
    fn run_reports_errors_as_toasts_and_keeps_the_session() {
        let mut app = MaterialCraftApp::headless(sample_session(), UiState::default());
        assert!(app.run("no.such.command", json!({})).is_none());
        assert!(app.toasts.iter().any(|t| t.error));
        assert!(app.run("graph.node.add", json!({"def": "levels"})).is_some());
    }

    #[test]
    fn guarded_execute_returns_command_errors_as_strings() {
        let mut s = sample_session();
        let e = execute_guarded(&mut s, "no.such.command", &json!({})).unwrap_err();
        assert!(!e.is_empty());
        assert!(execute_guarded(&mut s, "graph.node.add", &json!({"def": "levels"})).is_ok());
    }
}
