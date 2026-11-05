use egui::Ui;
use materialcraft_engine::{core::FunctionGraph, nodes::function::definition};

use super::{FunctionTarget, canvas};
use crate::{MaterialCraftApp, graph::node_selection, i18n::tr};

fn current(app: &MaterialCraftApp) -> Option<(&FunctionGraph, u32)> {
    let editor = app.function_editor.as_ref()?;
    if editor.target != FunctionTarget::Graph(app.session.graph) {
        return None;
    }

    let graph = app.session.graph().ok()?.function.as_ref()?;
    let id = node_selection::inspected(&editor.selection, editor.inspected_node)?;
    graph.node(id)?;
    Some((graph, id))
}

pub(crate) fn title(app: &MaterialCraftApp) -> Option<String> {
    let (graph, id) = current(app)?;
    let node = graph.node(id)?;
    let label = definition(&node.op).map_or(node.op.as_str(), |definition| definition.label);
    Some(format!("{label} - {}", tr("Properties")))
}

pub(crate) fn show(app: &mut MaterialCraftApp, ui: &mut Ui) -> bool {
    let Some((graph, id)) = current(app) else { return false };
    let mut graph = graph.clone();
    let Some(node) = graph.nodes.iter_mut().find(|node| node.id == id) else { return false };

    let label = definition(&node.op).map_or(node.op.as_str(), |definition| definition.label);
    ui.heading(tr(label));
    if graph.output == Some(id) {
        ui.label(tr("Output node"));
    }

    let changed = node.value.as_mut().is_some_and(|value| crate::value_editor::show(ui, value));
    if changed && let Some(editor) = app.function_editor.take() {
        canvas::commit(app, &editor, &graph);
        app.function_editor = Some(editor);
    }
    true
}
