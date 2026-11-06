use crate::{identifier, instance_graph};
use materialcraft_core::{Node, Package, Value};
use materialcraft_image::Image;
use std::collections::{BTreeMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};

pub(crate) fn image_key(i: &Image) -> u64 {
    let step = (i.data.len() / 4096).max(1);
    hash_of(&(i.w, i.h, i.data.iter().step_by(step).map(|v| v.to_bits()).collect::<Vec<u32>>()))
}

pub(crate) fn hash_of(v: &impl Hash) -> u64 {
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

pub(crate) fn graph_tree_hash(pkg: &Package, gi: usize) -> u64 {
    let mut h = DefaultHasher::new();
    let mut pending = vec![gi];
    let mut seen = HashSet::new();
    while let Some(index) = pending.pop() {
        if !seen.insert(index) {
            continue;
        }
        let Some(graph) = pkg.graphs.get(index) else { continue };

        index.hash(&mut h);
        serde_json::to_string(graph).unwrap_or_default().hash(&mut h);
        for n in &graph.nodes {
            if let Some(Value::Text(name)) = n.params.get("resource") {
                pkg.resource(name).map(|r| r.bytes.content_hash()).hash(&mut h);
            }
            if n.def == "graph_instance"
                && let Some((si, _)) = instance_graph(pkg, n)
            {
                pending.push(si);
            }
        }
    }
    h.finish()
}

pub(crate) fn variable_key(node: &Node, parameters: &BTreeMap<String, Value>, variables: &BTreeMap<String, Value>) -> u64 {
    let mut names = std::collections::BTreeSet::new();
    let mut pending: Vec<&Value> = parameters.values().collect();
    while let Some(value) = pending.pop() {
        match value {
            Value::Function(graph) => {
                for function in &graph.nodes {
                    if (function.op.starts_with("get_") || function.op == "isdefined")
                        && let Some(Value::Text(name)) = &function.value
                    {
                        names.insert(name.as_str());
                    }
                }
            }
            Value::PatternGraph(graph) => {
                pending.extend(graph.nodes.iter().flat_map(|node| node.parameters.values()));
            }
            _ => {}
        }
    }

    let input_identifier = identifier(node);
    if node.def == "input_value" {
        names.insert(&input_identifier);
    }

    let mut hasher = DefaultHasher::new();
    for name in names {
        name.hash(&mut hasher);
        variables.get(name).map(|value| serde_json::to_string(value).unwrap_or_default()).hash(&mut hasher);
    }
    hasher.finish()
}
