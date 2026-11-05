use egui::Ui;
use materialcraft_engine::core::PatternNodeKind;

use crate::{MaterialCraftApp, i18n::tr};

pub(crate) fn show(app: &mut MaterialCraftApp, ui: &mut Ui) {
    ui.label(tr("FX-Map nodes"));
    for kind in PatternNodeKind::ALL {
        if ui.selectable_label(false, tr(kind.label())).double_clicked()
            && let Some(mut editor) = app.pattern_editor.take()
        {
            if let Some(mut graph) = editor.current(app) {
                editor.menu_at = (editor.pan + app.graph.view.size() * 0.5 / editor.zoom).to_pos2();
                match editor.add(&mut graph, kind, app.ui.prefs.snap_to_grid) {
                    Ok(()) => editor.commit(app, graph),
                    Err(error) => app.toast(error, true),
                }
            }
            app.pattern_editor = Some(editor);
        }
    }
}
