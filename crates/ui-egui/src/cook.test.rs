#[cfg(test)]
mod cook_tests {
    use crate::MaterialCraftApp;
    use crate::cook::{drive, start};
    use crate::state::UiState;
    use materialcraft_engine::{execute, sample_session};
    use serde_json::json;

    fn wait_and_drive(app: &mut MaterialCraftApp, ctx: &egui::Context) {
        for _ in 0..2000 {
            drive(app, ctx);
            if !app.cooker.busy() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("background cook never finished");
    }

    /// A background cook of version A lands after a synchronous cook of version B (as in
    /// `session.inspect`): B's results must survive.
    #[test]
    fn stale_background_cook_does_not_overwrite_newer_sync_cook() {
        let mut session = sample_session();
        session.preview_size = Some(64);
        let mut app = MaterialCraftApp::headless(session, UiState::default());
        let ctx = egui::Context::default();

        start(&mut app, 0.0); // cook of version A runs in the background
        let added = execute(&mut app.session, "graph.node.add", &json!({"def": "clouds_1", "at": [0, 0]})).unwrap();
        let id = materialcraft_engine::core::NodeId(added["id"].as_u64().unwrap());
        assert!(app.session.dirty);
        app.session.cook_if_dirty(); // version B cooked synchronously, dirty cleared
        assert!(app.session.cooked.nodes.contains_key(&id));

        wait_and_drive(&mut app, &ctx);
        assert!(app.session.cooked.nodes.contains_key(&id), "stale cook replaced the newer results");
    }
}
