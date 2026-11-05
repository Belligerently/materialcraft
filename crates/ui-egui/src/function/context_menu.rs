use super::{FunctionEditor, FunctionTarget, selection};
use crate::{MaterialCraftApp, i18n::tr, menus};
use egui::Ui;
use materialcraft_engine::{
    core::{FunctionGraph, FunctionNode},
    nodes::function::{FUNCTIONS, default_value},
};
use serde_json::json;

pub(super) fn show(app: &mut MaterialCraftApp, editor: &mut FunctionEditor, graph: &mut FunctionGraph, ui: &mut Ui) -> bool {
    let mut changed = false;
    let mut add = None;
    let standalone = matches!(editor.target, FunctionTarget::Graph(_));
    ui.set_min_width(290.0);
    ui.menu_button(tr("Add node"), |ui| {
        let entries = FUNCTIONS.iter().filter(|operation| operation.id != "passthrough").map(|operation| (operation.id, operation.label, operation.category));
        add = crate::graph::add_node_menu::show(ui, entries);
    });

    if standalone {
        for (label, command, parameters) in [
            ("Add Comment", "graph.comment.add", json!({"at": [editor.menu_at.x, editor.menu_at.y]})),
            ("Add Frame", "graph.frame.add", json!({"rect": [editor.menu_at.x, editor.menu_at.y, 400, 260]})),
            ("Add Navigation pin", "graph.pin.add", json!({"at": [editor.menu_at.x, editor.menu_at.y]})),
        ] {
            if ui.button(tr(label)).clicked() {
                app.run(command, parameters);
                editor.selection.clear();
                ui.close();
            }
        }
    }
    if ui.button(tr("Add Dot node")).clicked() {
        add = Some("passthrough");
        ui.close();
    }
    ui.separator();

    if ui.add(egui::Button::new(tr("Save package")).shortcut_text("Ctrl+S")).clicked() {
        menus::invoke(app, ui.ctx(), "file.save");
        ui.close();
    }
    for (label, command) in [
        ("Paste", "edit.paste"),
        ("Paste without link", "edit.pasteWithoutLinks"),
        ("Select all", "edit.selectAll"),
        ("Previous pin", "ui.graph.previousPin"),
        ("Next pin", "ui.graph.nextPin"),
        ("Copy selection", "edit.copy"),
        ("Delete selection", "edit.delete"),
        ("Delete and relink", "edit.deleteRelink"),
        ("Duplicate selection", "edit.duplicate"),
        ("Duplicate selection without link", "edit.duplicateWithoutLinks"),
        ("Select upstream nodes", "graph.selectUpstream"),
        ("Select downstream nodes", "graph.selectDownstream"),
        ("Disable selection", "graph.node.disable"),
    ] {
        if command == "edit.copy" {
            ui.separator();
        }
        let annotation = standalone && app.session.annotation_selection.is_some() && matches!(command, "edit.delete" | "edit.deleteRelink");
        let enabled = if annotation { true } else { selection::enabled(editor, command).unwrap_or_else(|| standalone && menus::enabled(app, command)) };
        let mut button = egui::Button::new(tr(label));
        if let Some(shortcut) = menus::shortcut_of(command) {
            button = button.shortcut_text(shortcut);
        }
        if ui.add_enabled(enabled, button).clicked() {
            if annotation {
                app.run(command, json!({}));
            } else if command.starts_with("ui.") {
                menus::invoke(app, ui.ctx(), command);
                editor.pan = app.ui.graph_camera.pan.into();
            } else {
                changed |= selection::run(editor, graph, command, app.ui.prefs.snap_to_grid);
            }
            ui.close();
        }
    }

    if let Some(operation) = add {
        let id = graph.next_id();
        let inputs = FUNCTIONS.iter().find(|definition| definition.id == operation).map_or(0, |definition| definition.inputs.len());
        graph.nodes.push(FunctionNode {
            id,
            type_code: 0,
            op: operation.into(),
            inputs: vec![None; inputs],
            value: default_value(operation),
            pos: materialcraft_engine::graph_grid::snap_position([editor.menu_at.x, editor.menu_at.y], app.ui.prefs.snap_to_grid),
        });
        editor.selection = vec![id];
        if graph.output.is_none() {
            graph.output = Some(id);
        }
        changed = true;
    }
    changed
}
