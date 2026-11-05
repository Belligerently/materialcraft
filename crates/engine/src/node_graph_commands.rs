use materialcraft_core::NodeId;
use serde_json::{Value, json};

use crate::commands::{CommandSpec, bad};
use crate::{Result, Session, cmd};

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        "graph.node.edit",
        "Edit node graph",
        ["Edit"],
        Some("Ctrl+E"),
        "{} resolves the selected FX-Map graph to {node, parameter}; UI invocation opens its editor",
        enabled,
        edit
    )]
}

fn selected_node(session: &Session) -> std::result::Result<NodeId, String> {
    if session.annotation_selection.is_some() {
        return Err("select an FX-Map node".into());
    }

    let id = session.selection.last().copied().ok_or("select an FX-Map node")?;
    let graph = session.graph().map_err(|error| error.to_string())?;
    let node = graph.node(id).map_err(|error| error.to_string())?;
    if node.def != "fx_map" {
        return Err("the selected node has no editable FX-Map graph".into());
    }

    Ok(id)
}

fn enabled(session: &Session) -> std::result::Result<(), String> {
    selected_node(session).map(|_| ())
}

fn edit(session: &mut Session, _: &Value) -> Result<Value> {
    let node = selected_node(session).map_err(|error| bad("graph.node.edit", error))?;
    Ok(json!({"node": node.0, "parameter": "pattern"}))
}
