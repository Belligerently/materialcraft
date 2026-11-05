use egui::Ui;
use materialcraft_engine::core::{FunctionGraph, Value};

use crate::{
    MaterialCraftApp,
    function::{FunctionEditor, FunctionTarget},
    i18n::tr,
};

pub(crate) fn title(app: &MaterialCraftApp) -> String {
    let kind = app.pattern_editor.as_ref().and_then(|editor| {
        let graph = editor.current(app)?;
        let inspected = crate::graph::node_selection::inspected(&editor.selection, editor.inspected_node);
        graph.nodes.iter().find(|node| Some(node.id) == inspected).map(|node| node.kind)
    });
    format!("{} - {}", kind.map_or("FX-Map", |kind| kind.label()), tr("Properties"))
}

pub(crate) fn show(app: &mut MaterialCraftApp, ui: &mut Ui) {
    let Some(editor) = app.pattern_editor.take() else { return };
    let Some(mut graph) = editor.current(app) else { return };
    let mut changed = false;
    let mut function_target = None;

    let inspected = crate::graph::node_selection::inspected(&editor.selection, editor.inspected_node);
    if let Some(node) = graph.nodes.iter_mut().find(|node| Some(node.id) == inspected) {
        ui.heading(tr(node.kind.label()));
        if graph.root == Some(node.id) {
            ui.label(tr("Root node"));
        }
        if let Some(definition) = materialcraft_engine::nodes::pattern::definition(node.kind) {
            for parameter in &definition.params {
                ui.push_id(parameter.id, |ui| {
                    ui.label(tr(parameter.label));
                    let mut value = node.parameters.get(parameter.id).unwrap_or(&parameter.default).clone();
                    ui.horizontal_wrapped(|ui| {
                        if matches!(value, Value::Function(_)) {
                            if ui.small_button(tr("Edit function")).clicked() {
                                function_target = Some(FunctionTarget::Pattern { node: node.id, parameter: parameter.id.to_string() });
                            }
                            if ui.small_button(tr("Reset")).clicked() {
                                node.parameters.remove(parameter.id);
                                changed = true;
                            }
                        } else {
                            if super::parameter_value::show(ui, parameter.id, &mut value) {
                                node.parameters.insert(parameter.id.to_string(), value.clone());
                                changed = true;
                            }
                            if ui.small_button(tr("Function")).clicked() {
                                node.parameters.insert(parameter.id.to_string(), Value::Function(Box::new(FunctionGraph::constant(value))));
                                function_target = Some(FunctionTarget::Pattern { node: node.id, parameter: parameter.id.to_string() });
                                changed = true;
                            }
                        }
                    });
                    ui.add_space(6.0);
                });
            }
        }
    } else {
        ui.heading(tr("FX-Map"));
        ui.label(tr("Select a node to edit its parameters"));
    }
    if changed {
        editor.commit(app, graph);
    }
    if let Some(target) = function_target {
        app.function_editor = Some(FunctionEditor::new(editor.node, target));
    }
    app.pattern_editor = Some(editor);
}
