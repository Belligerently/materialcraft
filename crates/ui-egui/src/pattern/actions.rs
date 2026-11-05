use super::selection;
use crate::MaterialCraftApp;

pub(crate) fn active(app: &MaterialCraftApp) -> bool {
    app.pattern_editor.as_ref().is_some_and(|editor| {
        editor.graph == app.session.graph && app.session.graph().is_ok_and(|graph| graph.node(editor.node).is_ok_and(|node| node.def == "fx_map"))
    })
}

pub(crate) fn enabled(app: &MaterialCraftApp, command: &str) -> Option<bool> {
    if command == "ui.graph.parent" {
        return Some(active(app));
    }
    if !active(app) {
        return None;
    }

    if let Some(enabled) = selection::enabled(app.pattern_editor.as_ref()?, command) {
        return Some(enabled);
    }
    match command {
        "ui.graph.snapToGrid" | "graph.cook" => None,
        command
            if command.starts_with("graph.")
                || command.starts_with("ui.graph.")
                || command.starts_with("edit.") && !matches!(command, "edit.undo" | "edit.redo") =>
        {
            Some(false)
        }
        _ => None,
    }
}

pub(crate) fn invoke(app: &mut MaterialCraftApp, command: &str) -> bool {
    if command == "ui.graph.parent" {
        app.pattern_editor = None;
        app.function_editor = None;
        return true;
    }
    let Some(allowed) = enabled(app, command) else { return false };
    if !allowed {
        return true;
    }

    let Some(mut editor) = app.pattern_editor.take() else { return true };
    if let Some(mut graph) = editor.current(app) {
        match selection::run(&mut editor, &mut graph, command, app.ui.prefs.snap_to_grid) {
            Ok(true) => editor.commit(app, graph),
            Ok(false) => {}
            Err(error) => app.toast(error, true),
        }
    }
    app.pattern_editor = Some(editor);
    true
}
