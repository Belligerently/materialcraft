use crate::commands::{CommandSpec, bad, has_graph};
use crate::{Result, Session, cmd};
use materialcraft_core::FunctionGraph;
use serde_json::{Value as Json, json};

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![cmd!("graph.function.set", "Edit Function Graph", [], None, "{function} replaces the current function graph", has_graph, set)]
}

fn set(session: &mut Session, parameters: &Json) -> Result<Json> {
    if session.graph()?.function.is_none() {
        return Err(bad("graph.function.set", "the current graph is not a Function Graph"));
    }

    let graph: FunctionGraph =
        serde_json::from_value(parameters.get("function").cloned().unwrap_or(Json::Null)).map_err(|error| bad("graph.function.set", error.to_string()))?;
    materialcraft_nodes::function::node_types(&graph).map_err(|error| bad("graph.function.set", error))?;

    session.edit("Edit Function Graph", |session| {
        session.graph_mut()?.function = Some(graph);
        Ok(json!({}))
    })
}
