//! Missing math primitives run through the same commands as the function editor and agents.
use materialcraft_engine::core::{NodeId, Value, function::FnGraph};
use materialcraft_engine::{Session, execute, formats, new_session};
use serde_json::json;

fn math_function() -> FnGraph {
    let mut f = FnGraph::default();
    let power = f.add("get_var", &[], Some(Value::Text("power".into())));
    let radius = f.add("pow2", &[power], None);
    let two = f.add("const_float", &[], Some(Value::Float(2.0)));
    let angle = f.add("log2", &[two], None);
    f.output = Some(f.add("cartesian", &[radius, angle], None));
    f
}

fn assert_images(s: &mut Session, nodes: &[(NodeId, f32)], radius: f32) {
    s.cook();
    for (node, alpha) in nodes {
        let result = &s.cooked.nodes[node];
        assert!(result.error.is_none(), "{:?}", result.error);
        let mean = s.cooked.image(*node, 0).unwrap().mean();
        assert!((mean[0] - radius * 1.0f32.cos()).abs() < 1e-5, "{mean:?}");
        assert!((mean[1] - radius * 1.0f32.sin()).abs() < 1e-5, "{mean:?}");
        assert!(mean[2].abs() < 1e-5 && (mean[3] - alpha).abs() < 1e-5);
    }
}

#[test]
fn math_primitives_cook_in_processors_and_parameter_functions_and_round_trip() {
    let mut s = new_session();
    s.preview_size = Some(64);
    execute(&mut s, "graph.exposed.add", &json!({"id": "power", "default": {"t": "float", "v": -2.0}})).unwrap();
    let add = |s: &mut Session, def| NodeId(execute(s, "graph.node.add", &json!({"def": def})).unwrap()["id"].as_u64().unwrap());
    let processor = add(&mut s, "value_processor");
    let pixel = add(&mut s, "pixel_processor");
    let uniform = add(&mut s, "uniform");
    let f = math_function();
    execute(&mut s, "graph.node.param", &json!({"id": processor.0, "param": "function", "value": Value::Function(Box::new(f.clone()))})).unwrap();
    execute(&mut s, "graph.node.param", &json!({"id": pixel.0, "param": "function", "value": Value::Function(Box::new(f.clone()))})).unwrap();
    execute(&mut s, "graph.node.function", &json!({"id": uniform.0, "param": "color", "function": f})).unwrap();
    let nodes = [(processor, 0.0), (pixel, 1.0), (uniform, 0.0)];
    assert_images(&mut s, &nodes, 0.25);
    execute(&mut s, "graph.exposed.set", &json!({"id": "power", "value": -1.0})).unwrap();
    assert_images(&mut s, &nodes, 0.5);
    execute(&mut s, "edit.undo", &json!({})).unwrap();
    assert_images(&mut s, &nodes, 0.25);
    execute(&mut s, "edit.redo", &json!({})).unwrap();
    assert_images(&mut s, &nodes, 0.5);

    // SBS preservation here uses MaterialCraft's embedded exact package, not external XML import.
    let native = formats::load_package(&formats::save_package(&s.pkg).unwrap()).unwrap();
    let sbs = formats::sbs::read_sbs(&formats::sbs::write_sbs(&s.pkg).unwrap(), "math.sbs", &|_| None).unwrap().pkg;
    for package in [native, sbs] {
        assert_eq!(package, s.pkg);
        let mut loaded = new_session();
        loaded.preview_size = Some(64);
        loaded.pkg = package;
        assert_images(&mut loaded, &nodes, 0.5);
    }
}
