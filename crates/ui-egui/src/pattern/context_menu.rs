use egui::Ui;
use materialcraft_engine::core::{PatternGraph, PatternNodeKind};

use super::{PatternEditor, selection};
use crate::{MaterialCraftApp, i18n::tr, menus};

pub(super) fn show(app: &mut MaterialCraftApp, editor: &mut PatternEditor, graph: &mut PatternGraph, ui: &mut Ui) -> bool {
    let mut changed = false;
    ui.menu_button(tr("Add node"), |ui| {
        let kinds = PatternNodeKind::ALL.into_iter().filter(|kind| *kind != PatternNodeKind::Dot);
        let entries = kinds.map(|kind| (kind.native_id(), kind.label(), "FX-Map"));
        if let Some(id) = crate::graph::add_node_menu::show(ui, entries)
            && let Some(kind) = PatternNodeKind::ALL.into_iter().find(|kind| kind.native_id() == id)
        {
            match editor.add(graph, kind, app.ui.prefs.snap_to_grid) {
                Ok(()) => changed = true,
                Err(error) => app.toast(error, true),
            }
        }
    });
    if ui.button(tr("Add Dot node")).clicked() {
        match editor.add(graph, PatternNodeKind::Dot, app.ui.prefs.snap_to_grid) {
            Ok(()) => changed = true,
            Err(error) => app.toast(error, true),
        }
        ui.close();
    }
    if let Some(id) = editor.selection.last().copied()
        && ui.button(tr("Set as root")).clicked()
    {
        graph.root = Some(id);
        changed = true;
        ui.close();
    }
    ui.separator();

    for (label, command) in [
        ("Copy selection", "edit.copy"),
        ("Cut selection", "edit.cut"),
        ("Paste", "edit.paste"),
        ("Paste without link", "edit.pasteWithoutLinks"),
        ("Duplicate selection", "edit.duplicate"),
        ("Duplicate selection without link", "edit.duplicateWithoutLinks"),
        ("Delete selection", "edit.delete"),
        ("Select all", "edit.selectAll"),
        ("Select upstream nodes", "graph.selectUpstream"),
        ("Select downstream nodes", "graph.selectDownstream"),
    ] {
        let mut button = egui::Button::new(tr(label));
        if let Some(shortcut) = menus::shortcut_of(command) {
            button = button.shortcut_text(shortcut);
        }
        if ui.add_enabled(selection::enabled(editor, command).unwrap_or(false), button).clicked() {
            match selection::run(editor, graph, command, app.ui.prefs.snap_to_grid) {
                Ok(edit) => changed |= edit,
                Err(error) => app.toast(error, true),
            }
            ui.close();
        }
    }
    ui.separator();
    if ui.button(tr("Frame all")).clicked() {
        editor.framed = false;
        ui.close();
    }
    if ui.add(egui::Button::new(tr("Save package")).shortcut_text("Ctrl+S")).clicked() {
        menus::invoke(app, ui.ctx(), "file.save");
        ui.close();
    }
    changed
}
