use std::collections::{BTreeMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use materialcraft_core::{Graph, Node, NodeId, Package, Value};
use materialcraft_image::{Image, Kind};
use materialcraft_nodes::{CookContext, clamp_params, find, resolve_params};

use crate::cache_key::{hash_of, image_key};
use crate::{Cache, Cooked, MAX_DEPTH, NodeOutput, NodeResult, Options, Port, identifier, instance_graph, node_size, ports};

/// The magenta image a failed node shows
pub fn error_image(w: u32, h: u32) -> Image {
    Image::filled(w, h, Kind::Color, [1.0, 0.0, 1.0, 1.0])
}

/// Lock the cache, recovering from a poisoned lock (a cache entry is never half-written)
fn lock(cache: &Mutex<Cache>) -> std::sync::MutexGuard<'_, Cache> {
    cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn cook(pkg: &Package, gi: usize, opts: &Options, cache: &Mutex<Cache>) -> Cooked {
    let t0 = now_ms();
    let mut out = Cooked::default();
    let Some(graph) = pkg.graphs.get(gi) else { return out };
    // The top-level graph's input nodes show their preview images; the caller's images win
    let bound;
    let opts = if opts.depth == 0 && !graph.input_images.is_empty() {
        let mut inputs: BTreeMap<String, Image> = graph.input_images.iter().filter_map(|(id, res)| Some((id.clone(), (opts.resource)(res)?))).collect();
        inputs.extend(opts.inputs.iter().map(|(k, v)| (k.clone(), v.clone())));
        bound = Options { only: opts.only.clone(), inputs, overrides: opts.overrides.clone(), graph_stack: opts.graph_stack.clone(), ..*opts };
        &bound
    } else {
        opts
    };

    let order = graph.topo_order();
    let needed: Option<std::collections::BTreeSet<NodeId>> = opts.only.as_ref().map(|ids| {
        let mut s: std::collections::BTreeSet<NodeId> = ids.iter().copied().collect();
        for id in ids {
            s.extend(graph.upstream(*id));
        }
        s
    });
    // Exposed parameter values: the graph's own, then the caller's overrides
    let mut vars: BTreeMap<String, Value> = graph.exposed.iter().filter_map(|e| graph.input_value(&e.id).map(|v| (e.id.clone(), v.clone()))).collect();
    for (k, v) in &opts.overrides {
        vars.insert(k.clone(), v.clone());
    }
    for id in order {
        if needed.as_ref().is_some_and(|s| !s.contains(&id)) {
            continue;
        }
        if opts.cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            out.cancelled = true;
            break;
        }
        let Ok(node) = graph.node(id) else { continue };
        let r = cook_node(pkg, gi, graph, node, &out, &vars, opts, cache);
        out.nodes.insert(id, r);
    }
    out.ms = (now_ms() - t0) as f32;
    out
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

fn cook_node(
    pkg: &Package,
    gi: usize,
    graph: &Graph,
    node: &Node,
    done: &Cooked,
    vars: &BTreeMap<String, Value>,
    opts: &Options,
    cache: &Mutex<Cache>,
) -> NodeResult {
    let (in_ports, out_ports) = ports(pkg, graph, node);
    let def = find(&node.def);
    // Gather inputs by port order, with their keys
    let mut inputs = Vec::new();
    let mut incoming = Vec::new();
    let mut input_keys = Vec::new();
    let mut variables = vars.clone();
    let mut input_error = None;
    for port in &in_ports {
        let source = graph.input_of(node.id, &port.id).and_then(|connection| {
            let result = done.nodes.get(&connection.from)?;
            let source_node = graph.node(connection.from).ok()?;
            let index = ports(pkg, graph, source_node).1.iter().position(|output| output.id == connection.from_port)?;
            Some((result.outputs.get(index)?.clone(), result.key ^ hash_of(&index)))
        });
        input_keys.push(source.as_ref().map_or(0, |source| source.1));
        let output = source.map(|source| source.0);

        if let Some(expected) = node.input_values.get(&port.id) {
            let value = match output.as_ref() {
                Some(NodeOutput::Value(value)) if materialcraft_core::ValueType::of(value) == Some(*expected) => value.clone(),
                Some(_) => {
                    input_error = Some(format!("input `{}` requires {}", port.id, expected.label()));
                    expected.default_value()
                }
                None => expected.default_value(),
            };

            variables.insert(port.id.clone(), value);
        } else {
            if output.as_ref().is_some_and(|output| output.as_value().is_some()) && !matches!(node.def.as_str(), "output" | "dot") {
                input_error = Some(format!("input `{}` requires an image", port.id));
            }
            inputs.push(output.as_ref().and_then(NodeOutput::as_image).cloned());
        }

        incoming.push(output);
    }
    let vars = &variables;
    let first = inputs.iter().flatten().next().map(|i| (i.w, i.h));
    let (w, h) = node_size(graph, def, node, first, opts.max_size);
    let fail = |msg: String, key: u64| NodeResult {
        outputs: Arc::new(vec![NodeOutput::Image(error_image(w, h)); out_ports.len().max(1)]),
        error: Some(msg),
        ms: 0.0,
        key,
        cached: false,
    };

    if let Some(error) = input_error {
        return fail(error, 0);
    }

    // Parameters: defaults, overrides, then bindings to exposed inputs
    let mut params = match def {
        Some(d) if node.def != "graph_instance" => resolve_params(d, &node.params),
        _ => node.params.clone(),
    };

    for (param, exposed) in &node.bindings {
        if let (Some(v), Some(cur)) = (vars.get(exposed), params.get(param))
            && let Some(c) = cur.coerce_json(&serde_json::to_value(v).unwrap_or_default())
        {
            params.insert(param.clone(), c);
        }
    }
    let seed = (graph.seed as u32).wrapping_mul(0x9e37_79b9).wrapping_add(node.base.seed as u32);
    // Dynamic parameters: functions of the graph's inputs, evaluated once per cook
    for (param, f) in &node.functions {
        let Some(cur) = params.get(param) else { continue };
        match materialcraft_nodes::function::compile(f, seed) {
            Ok(prog) => {
                let context = materialcraft_nodes::function::EvaluationContext {
                    position: [0.5; 2],
                    size: [w as f32, h as f32],
                    images: &[],
                    tile: true,
                    variables: vars,
                };

                let value = match prog.evaluate(&context, &mut materialcraft_nodes::function::EvaluationState::default()) {
                    Ok(value) => value,
                    Err(error) => return fail(format!("function of `{param}`: {error}"), 0),
                };

                if let Some(c) = cur.coerce_json(&serde_json::to_value(value).unwrap_or_default()) {
                    params.insert(param.clone(), c);
                }
            }
            Err(e) => return fail(format!("function of `{param}`: {e}"), 0),
        }
    }
    // Hard limits on whatever files, bindings and functions supplied (a value can't hang a cook)
    if let Some(d) = def
        && node.def != "graph_instance"
    {
        clamp_params(d, &mut params);
    }
    let tile = node.base.tiling[0] && node.base.tiling[1];
    // Input nodes read the caller's image for their identifier
    let input_img = node.def.starts_with("input_").then(|| opts.inputs.get(&identifier(node))).flatten();
    let res_hash = match params.get("resource") {
        Some(Value::Text(name)) => pkg.resource(name).map_or(0, |r| r.bytes.content_hash()),
        _ => 0,
    };

    let sub_hash = if node.def == "graph_instance" { instance_graph(pkg, node).map_or(0, |(si, _)| graph_tree_hash(pkg, si)) } else { 0 };
    let key = hash_of(&(
        &node.def,
        serde_json::to_string(&params).unwrap_or_default(),
        (w, h, seed, tile, node.disabled),
        &input_keys,
        res_hash,
        sub_hash,
        input_img.map(image_key),
        variable_key(node, &params, vars),
    ));
    if let Some(v) = lock(cache).get(key) {
        return NodeResult { outputs: v, error: None, ms: 0.0, key, cached: true };
    }
    let t0 = now_ms();
    let result: Result<Vec<NodeOutput>, String> = if node.disabled {
        let output = incoming.iter().flatten().next().cloned().unwrap_or_else(|| NodeOutput::Image(Image::filled(w, h, Kind::Gray, [0.0; 4])));
        Ok(vec![output; out_ports.len().max(1)])
    } else if node.def == "input_value" {
        let value = vars.get(&identifier(node)).or_else(|| params.get("default")).cloned().unwrap_or(Value::Float(0.0));
        Ok(vec![NodeOutput::Value(value)])
    } else if node.def == "value_processor" {
        evaluate_value(&params, vars, seed, [w, h], &inputs, tile).map(|value| vec![NodeOutput::Value(value)])
    } else if matches!(node.def.as_str(), "output" | "dot") && incoming.first().and_then(Option::as_ref).is_some_and(|output| output.as_value().is_some()) {
        incoming.first().and_then(Clone::clone).map(|output| vec![output]).ok_or("missing value input".into())
    } else if let Some(image) = input_img {
        let image = image.resize(w, h, tile);
        let image = if node.def == "input_grayscale" { image.to_gray() } else { image };
        Ok(vec![NodeOutput::Image(image)])
    } else if node.def == materialcraft_core::MISSING_NODE {
        let name = if node.label.is_empty() { "this node" } else { node.label.as_str() };
        Err(format!("unsupported node `{name}`"))
    } else if node.def == "graph_instance" {
        cook_instance(pkg, gi, node, &in_ports, &out_ports, &inputs, &params, (w, h), opts, cache)
            .map(|images| images.into_iter().map(NodeOutput::Image).collect())
    } else if let Some(definition) = def {
        let context = CookContext { w, h, seed, tile, inputs: &inputs, params: &params, resource: opts.resource, vars };
        (definition.cook)(&context).map(|images| images.into_iter().map(NodeOutput::Image).collect())
    } else {
        Err(format!("unknown node `{}`", node.def))
    };

    match result {
        Ok(imgs) if imgs.len() == out_ports.len().max(1) || out_ports.is_empty() => {
            let v = Arc::new(imgs);
            lock(cache).put(key, v.clone());
            NodeResult { outputs: v, error: None, ms: (now_ms() - t0) as f32, key, cached: false }
        }
        Ok(imgs) => fail(format!("`{}` made {} images for {} outputs", node.def, imgs.len(), out_ports.len()), key),
        Err(e) => fail(e, key),
    }
}

/// Whether graph `from` reaches `target`, without recursive stack growth
pub(crate) fn instances_reach(pkg: &Package, from: usize, target: usize) -> bool {
    let mut pending = vec![from];
    let mut seen = HashSet::new();
    while let Some(index) = pending.pop() {
        if !seen.insert(index) {
            continue;
        }
        let Some(graph) = pkg.graphs.get(index) else { continue };
        for (si, _) in graph.nodes.iter().filter(|n| n.def == "graph_instance").filter_map(|n| instance_graph(pkg, n)) {
            if si == target {
                return true;
            }
            pending.push(si);
        }
    }
    false
}

fn cook_instance(
    pkg: &Package,
    gi: usize,
    node: &Node,
    in_ports: &[Port],
    out_ports: &[Port],
    inputs: &[Option<Image>],
    params: &BTreeMap<String, Value>,
    size: (u32, u32),
    opts: &Options,
    cache: &Mutex<Cache>,
) -> Result<Vec<Image>, String> {
    if opts.depth >= MAX_DEPTH {
        return Err("graph instances nest too deep (does a graph use itself?)".into());
    }
    let (sub_gi, sub) = instance_graph(pkg, node).ok_or("the instanced graph does not exist")?;
    if sub_gi == gi || opts.graph_stack.contains(&sub_gi) || instances_reach(pkg, sub_gi, gi) {
        return Err(format!("graph `{}` uses itself through a graph instance", sub.name));
    }
    let mut o = Options::new(opts.resource);
    o.max_size = opts.max_size;
    o.cancel = opts.cancel;
    o.depth = opts.depth + 1;
    o.graph_stack = opts.graph_stack.clone();
    o.graph_stack.push(gi);
    o.inputs = in_ports.iter().zip(inputs).filter_map(|(p, i)| Some((p.id.clone(), i.clone()?))).collect();
    o.overrides = params.iter().filter(|(k, _)| sub.exposed.iter().any(|e| &e.id == *k)).map(|(k, v)| (k.clone(), v.clone())).collect();
    let cooked = cook(pkg, sub_gi, &o, cache);
    if cooked.cancelled {
        return Err("cancelled".into());
    }
    Ok(out_ports
        .iter()
        .map(|p| {
            sub.nodes
                .iter()
                .find(|n| n.def == "output" && identifier(n) == p.id)
                .and_then(|n| cooked.image(n.id, 0))
                .map(|i| i.resize(size.0, size.1, true))
                .unwrap_or_else(|| Image::filled(size.0, size.1, Kind::Gray, [0.0; 4]))
        })
        .collect())
}

/// The graph's output nodes with their usage id and image, in node order
pub fn outputs<'c>(graph: &Graph, cooked: &'c Cooked) -> Vec<(NodeId, &'static str, String, Option<&'c Image>)> {
    graph
        .nodes
        .iter()
        .filter(|n| n.def == "output")
        .map(|n| {
            let usage = n.params.get("usage").and_then(Value::as_i32).unwrap_or(0).clamp(0, materialcraft_nodes::atomic::USAGE_IDS.len() as i32 - 1) as usize;
            (n.id, materialcraft_nodes::atomic::USAGE_IDS.get(usage).copied().unwrap_or("any"), identifier(n), cooked.image(n.id, 0))
        })
        .collect()
}

/// Milliseconds since the first call (0 on the web, where `std::time::Instant` is unavailable)
pub fn now_ms() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.0
    }
    #[cfg(target_arch = "wasm32")]
    0.0
}

fn evaluate_value(
    parameters: &BTreeMap<String, Value>,
    variables: &BTreeMap<String, Value>,
    seed: u32,
    size: [u32; 2],
    images: &[Option<Image>],
    tile: bool,
) -> Result<Value, String> {
    use materialcraft_nodes::function::{EvaluationContext, EvaluationState, compile};

    let Some(Value::Function(graph)) = parameters.get("function") else {
        return Err("missing value function".into());
    };

    let program = compile(graph, seed)?;
    let context = EvaluationContext { position: [0.5; 2], size: size.map(|side| side as f32), images, tile, variables };

    program.evaluate(&context, &mut EvaluationState::default())
}

fn variable_key(node: &Node, parameters: &BTreeMap<String, Value>, variables: &BTreeMap<String, Value>) -> u64 {
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
