//! Parameter resolution tests.

use std::collections::BTreeMap;

use materialcraft_core::Value;

use crate::{catalog, clamp_params, find, resolve_params};

fn over(k: &str, v: Value) -> BTreeMap<String, Value> {
    BTreeMap::from([(k.to_string(), v)])
}

#[test]
fn resolve_params_clamps_to_declared_ranges() {
    let def = find("splatter").unwrap();
    let p = resolve_params(def, &over("size", Value::Float(1e30)));
    assert_eq!(p["size"], Value::Float(1.0));
    let p = resolve_params(def, &over("size", Value::Float(-5.0)));
    assert_eq!(p["size"], Value::Float(0.0));
    let p = resolve_params(def, &over("count", Value::Int(i32::MAX)));
    assert_eq!(p["count"], Value::Int(2000));
    let p = resolve_params(def, &over("size", Value::Float(f32::NAN)));
    assert_eq!(p["size"], def.param("size").unwrap().default);
    let shape = find("shape").unwrap();
    let p = resolve_params(shape, &over("size_xy", Value::Float2([1e9, -1e9])));
    assert_eq!(p["size_xy"], Value::Float2([2.0, 0.0]));
}

#[test]
fn clamp_params_covers_values_set_after_resolution() {
    let def = find("splatter").unwrap();
    let mut p = resolve_params(def, &BTreeMap::new());
    p.insert("size".into(), Value::Float(1e9));
    clamp_params(def, &mut p);
    assert_eq!(p["size"], Value::Float(1.0));
}

#[test]
fn defaults_lie_inside_their_declared_ranges() {
    for def in catalog() {
        let p = resolve_params(def, &BTreeMap::new());
        for d in &def.params {
            assert_eq!(p[d.id], d.default, "{}.{} default is outside its range", def.id, d.id);
        }
    }
}
