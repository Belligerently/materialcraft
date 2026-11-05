use crate::{MaterialCraftApp, i18n::tr};
use egui::Ui;
use materialcraft_engine::nodes::function::{FUNCTIONS, default_value};
use serde_json::json;

pub(crate) fn show(app: &mut MaterialCraftApp, ui: &mut Ui) {
    ui.add(egui::TextEdit::singleline(&mut app.ui.library_search).hint_text(tr("Search functions")));
    let search = app.ui.library_search.to_ascii_lowercase();
    egui::ScrollArea::vertical().show(ui, |ui| {
        for definition in FUNCTIONS.iter().filter(|definition| definition.label.to_ascii_lowercase().contains(&search)) {
            if ui.selectable_label(false, tr(definition.label)).double_clicked()
                && let Some(mut graph) = app.session.graph().ok().and_then(|graph| graph.function.clone())
            {
                let id = graph.add(definition.id, &[], default_value(definition.id));
                if graph.output.is_none() {
                    graph.output = Some(id);
                }
                if let Some(node) = graph.nodes.iter_mut().find(|node| node.id == id) {
                    node.pos = materialcraft_engine::graph_grid::snap_position(node.pos, app.ui.prefs.snap_to_grid);
                }
                app.run("graph.function.set", json!({"function": graph}));
            }
        }
    });
}
