use materialcraft_engine::{Session, core::NodeId, execute, formats, new_session};
use serde_json::{Value, json};

fn run(s: &mut Session, id: &str, params: Value) -> Value {
    execute(s, id, &params).unwrap()
}

#[test]
fn grouping_noise_preserves_pixels_seed_and_undo() {
    for seed in [0, 42, 12345] {
        let mut s = new_session();
        s.preview_size = None;
        run(&mut s, "graph.size", json!({"width": 64, "height": 32}));
        run(&mut s, "graph.seed", json!({"seed": seed}));
        let noise = run(&mut s, "graph.node.add", json!({"def": "perlin_noise"}))["id"].as_u64().unwrap();
        run(&mut s, "graph.node.base", json!({"id": noise, "seed": 7}));
        let output = NodeId(run(&mut s, "graph.node.add", json!({"def": "output", "connectFrom": {"node": noise}}))["id"].as_u64().unwrap());
        run(&mut s, "graph.select", json!({"ids": [noise]}));
        let before_graphs = s.pkg.graphs.clone();
        let before = s.cook().image(output, 0).unwrap().clone();
        run(&mut s, "graph.createFromSelection", json!({"name": "Grouped_noise"}));
        // Force an actual cook instead of relying on the pre-grouping cache entry.
        s.cache.lock().unwrap().clear();
        let after = s.cook().image(output, 0).unwrap().clone();
        assert!(before.data == after.data, "grouping changed the material at graph seed {seed}");
        assert_eq!((after.w, after.h), (64, 32));
        assert_eq!(s.pkg.graphs[1].seed, seed);
        let grouped_graphs = s.pkg.graphs.clone();
        run(&mut s, "edit.undo", json!({}));
        assert_eq!(s.pkg.graphs, before_graphs);
        assert_eq!(s.cook().image(output, 0).unwrap().data, before.data);
        run(&mut s, "edit.redo", json!({}));
        assert_eq!(s.pkg.graphs, grouped_graphs);
        assert_eq!(s.cook().image(output, 0).unwrap().data, before.data);
        let bytes = formats::save_package(&s.pkg).unwrap();
        let mut reopened = Session::new(formats::load_package(&bytes).unwrap());
        reopened.preview_size = None;
        assert_eq!(reopened.cook().image(output, 0).unwrap().data, before.data);
    }
}
