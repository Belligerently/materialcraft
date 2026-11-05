//! The reported library-node aliases must preserve imported settings and connections.
use materialcraft_core::{Graph, Package, Value};
use materialcraft_formats::sbs::{read_sbs, write_sbs};

fn plain_xml(bytes: &[u8]) -> String {
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    // Exercise XML import rather than the writer's embedded exact native-package copy.
    format!("{}</package>", text.split("<!--").next().unwrap())
}
fn fixture(def: &str) -> String {
    let mut pkg = Package::new("Import check");
    let mut graph = Graph::new("G");
    let source = graph.add_node("uniform", [0.0, 0.0]);
    let filter = graph.add_node(def, [150.0, 0.0]);
    graph.connect(source, "output", filter, "input").unwrap();
    if def == "histogram_scan" {
        let n = graph.node_mut(filter).unwrap();
        n.params.insert("position".into(), Value::Float(0.25));
        n.params.insert("contrast".into(), Value::Float(0.75));
    }
    pkg.graphs.push(graph);
    plain_xml(&write_sbs(&pkg).unwrap())
}
fn aliased_fixture(def: &str) -> String {
    let xml = fixture(def);
    if def == "histogram_scan" {
        xml.replace("<name v=\"position\"/>", "<name v=\"Position\"/>")
            .replace("<name v=\"contrast\"/>", "<name v=\"Contrast\"/>")
            .replace("<identifier v=\"input\"/>", "<identifier v=\"Input_1\"/>")
    } else {
        xml.replace("<identifier v=\"input\"/>", "<identifier v=\"Input\"/>")
    }
}
fn assert_wire(pkg: &Package, def: &str) {
    let g = &pkg.graphs[0];
    let source = g.nodes.iter().find(|n| n.def == "uniform").unwrap();
    let filter = g.nodes.iter().find(|n| n.def == def).unwrap();
    assert_eq!(g.connections.len(), 1);
    let wire = &g.connections[0];
    assert_eq!((wire.from, wire.from_port.as_str(), wire.to, wire.to_port.as_str()), (source.id, "output", filter.id, "input"));
}
fn assert_scan_values(pkg: &Package) {
    let n = pkg.graphs[0].nodes.iter().find(|n| n.def == "histogram_scan").unwrap();
    assert_eq!(n.params.get("position"), Some(&Value::Float(0.25)));
    assert_eq!(n.params.get("contrast"), Some(&Value::Float(0.75)));
    assert!(!n.params.contains_key("Position") && !n.params.contains_key("Contrast"));
}

#[test]
fn canonical_names_keep_parameters_and_wires() {
    for def in ["histogram_scan", "auto_levels"] {
        let read = read_sbs(fixture(def).as_bytes(), "check.sbs", &|_| None).unwrap();
        assert!(!read.exact);
        assert!(read.warnings.is_empty(), "{:?}", read.warnings);
        assert_wire(&read.pkg, def);
        if def == "histogram_scan" {
            assert_scan_values(&read.pkg);
        }
    }
}
#[test]
fn histogram_scan_keeps_reported_parameter_spellings() {
    let xml = fixture("histogram_scan").replace("<name v=\"position\"/>", "<name v=\"Position\"/>").replace("<name v=\"contrast\"/>", "<name v=\"Contrast\"/>");
    let read = read_sbs(xml.as_bytes(), "check.sbs", &|_| None).unwrap();
    assert!(read.warnings.is_empty(), "{:?}", read.warnings);
    assert_scan_values(&read.pkg);
}
#[test]
fn histogram_scan_keeps_reported_input_connection() {
    let xml = fixture("histogram_scan").replace("<identifier v=\"input\"/>", "<identifier v=\"Input_1\"/>");
    let read = read_sbs(xml.as_bytes(), "check.sbs", &|_| None).unwrap();
    assert!(read.warnings.is_empty(), "{:?}", read.warnings);
    assert_wire(&read.pkg, "histogram_scan");
}
#[test]
fn auto_levels_keeps_reported_input_connection() {
    let xml = fixture("auto_levels").replace("<identifier v=\"input\"/>", "<identifier v=\"Input\"/>");
    let read = read_sbs(xml.as_bytes(), "check.sbs", &|_| None).unwrap();
    assert!(read.warnings.is_empty(), "{:?}", read.warnings);
    assert_wire(&read.pkg, "auto_levels");
}
#[test]
fn aliased_imports_survive_a_structural_save_and_reload() {
    for def in ["histogram_scan", "auto_levels"] {
        let read = read_sbs(aliased_fixture(def).as_bytes(), "check.sbs", &|_| None).unwrap();
        assert!(read.warnings.is_empty(), "{:?}", read.warnings);
        let xml = plain_xml(&write_sbs(&read.pkg).unwrap());
        let back = read_sbs(xml.as_bytes(), "saved.sbs", &|_| None).unwrap();
        assert!(!back.exact);
        assert!(back.warnings.is_empty(), "{:?}", back.warnings);
        assert_wire(&back.pkg, def);
        if def == "histogram_scan" {
            assert_scan_values(&back.pkg);
        }
    }
}
#[test]
fn unknown_parameters_still_warn() {
    let xml = fixture("histogram_scan").replace("<name v=\"contrast\"/>", "<name v=\"UnexpectedContrast\"/>");
    let read = read_sbs(xml.as_bytes(), "check.sbs", &|_| None).unwrap();
    assert_eq!(read.warnings.len(), 1);
    assert!(read.warnings[0].contains("UnexpectedContrast"));
    let n = read.pkg.graphs[0].nodes.iter().find(|n| n.def == "histogram_scan").unwrap();
    assert_eq!(n.params.get("position"), Some(&Value::Float(0.25)));
    assert!(!n.params.contains_key("contrast") && !n.params.contains_key("UnexpectedContrast"));
}
#[test]
fn unknown_inputs_still_warn_and_drop_the_wire() {
    for def in ["histogram_scan", "auto_levels"] {
        let xml = fixture(def).replace("<identifier v=\"input\"/>", "<identifier v=\"UnexpectedInput\"/>");
        let read = read_sbs(xml.as_bytes(), "check.sbs", &|_| None).unwrap();
        assert_eq!(read.warnings.len(), 1);
        assert!(read.warnings[0].contains("UnexpectedInput"));
        assert!(read.pkg.graphs[0].connections.is_empty());
        assert_eq!(read.pkg.graphs[0].nodes.len(), 2);
    }
}
