//! The package tree, its graphs, named outputs and resources

use egui::{RichText, Ui};

mod tree_row;
use egui_phosphor::regular as icon;
use serde_json::json;

use crate::MaterialCraftApp;
use crate::i18n::tr;
use crate::theme::Tokens;

pub fn show(app: &mut MaterialCraftApp, ui: &mut Ui) {
    let tk = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        if crate::widgets::icon_button(ui, icon::FILE_PLUS, tr("New graph..."), false, 22.0).clicked() {
            app.ui.windows.new_graph = true;
        }
        if crate::widgets::icon_button(ui, icon::FOLDER_OPEN, tr("Open package..."), false, 22.0).clicked() {
            crate::menus::invoke(app, ui.ctx(), "file.open");
        }
        if crate::widgets::icon_button(ui, icon::FLOPPY_DISK, tr("Save package"), false, 22.0).clicked() {
            crate::menus::invoke(app, ui.ctx(), "file.save");
        }
        if crate::widgets::icon_button(ui, icon::EXPORT, tr("Export outputs..."), false, 22.0).clicked() {
            app.ui.windows.export = true;
        }
        if crate::widgets::icon_button(ui, icon::IMAGE, tr("Import bitmap..."), false, 22.0).clicked() {
            crate::menus::invoke(app, ui.ctx(), "file.importBitmap");
        }
    });
    ui.add_space(2.0);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let name = format!("{}{}", app.session.pkg.name, if app.session.modified { "*" } else { "" });
        ui.label(RichText::new(format!("{}  {name}", icon::PACKAGE)).strong().color(tk.text_strong));
        let mut open: Option<usize> = None;
        let mut action: Option<(&str, usize)> = None;
        let mut output = None;
        for (index, graph) in app.session.pkg.graphs.iter().enumerate() {
            let active = index == app.session.graph;
            let function = graph.function.is_some();
            let mut header = |ui: &mut Ui| {
                let symbol = if function { icon::FUNCTION } else { icon::TREE_STRUCTURE };
                let response = tree_row::show(ui, &graph.name, symbol, active);
                if response.clicked() {
                    open = Some(index);
                }
                response.context_menu(|ui| {
                    for (label, command) in [("Open", "open"), ("Export outputs...", "export"), ("Duplicate", "duplicate"), ("Delete", "delete")] {
                        if ui.button(tr(label)).clicked() {
                            action = Some((command, index));
                            ui.close();
                        }
                    }
                });
            };

            if function {
                ui.horizontal(|ui| {
                    ui.add_space(ui.spacing().indent);
                    header(ui);
                });
                continue;
            }

            let id = ui.make_persistent_id(("graph", index, &graph.name));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true).show_header(ui, header).body(|ui| {
                ui.indent("outputs", |ui| {
                    for node in graph.nodes.iter().filter(|node| node.def == "output") {
                        ui.push_id(node.id, |ui| {
                            let name = materialcraft_engine::eval::identifier(node);
                            let selected = active && app.session.selection.contains(&node.id);
                            let response = tree_row::show(ui, &name, icon::IMAGE, selected);

                            if response.clicked() || response.double_clicked() {
                                output = Some((index, node.id, response.double_clicked()));
                            }
                        });
                    }
                });
            });
        }
        if let Some(i) = open {
            app.run("graph.open", json!({"graph": i}));
        }
        if let Some((index, node, preview)) = output {
            if index != app.session.graph && app.run("graph.open", json!({"graph": index})).is_none() {
                return;
            }

            app.pattern_editor = None;
            app.function_editor = None;
            app.run("graph.annotation.select", json!({}));
            app.run("graph.select", json!({"ids": [node.0]}));
            app.graph.inspected_node = Some(node);
            app.ui.dock.focus(crate::docking::Panel::Properties);
            if preview {
                app.run("view.2d", json!({"node": node.0}));
                app.ui.dock.focus(crate::docking::Panel::View2d);
            }
        }

        match action {
            Some(("open", i)) => {
                app.run("graph.open", json!({"graph": i}));
            }
            Some(("export", i)) => {
                // The export window works on the open graph
                if app.run("graph.open", json!({"graph": i})).is_some() {
                    app.ui.windows.export = true;
                }
            }
            Some(("duplicate", i)) => {
                app.run("graph.duplicate", json!({"graph": i}));
            }
            Some(("delete", i)) => {
                app.run("graph.delete", json!({"graph": i}));
            }
            _ => {}
        }
        if !app.session.pkg.resources.is_empty() {
            ui.add_space(6.0);
            ui.label(RichText::new(tr("Resources")).color(tk.text_dim).size(11.0));
            for r in &app.session.pkg.resources {
                ui.label(format!("{}  {}  ({} KB)", icon::IMAGE, r.name, r.bytes.len() / 1024));
            }
        }
    });
}
