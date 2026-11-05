use super::FunctionEditor;
use materialcraft_engine::core::{FunctionGraph, FunctionNode};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn enabled(editor: &FunctionEditor, command: &str) -> Option<bool> {
    Some(match command {
        "edit.selectAll" | "edit.selectNone" => true,
        "edit.paste" | "edit.pasteWithoutLinks" => !editor.clipboard.is_empty(),
        "edit.copy"
        | "edit.cut"
        | "edit.delete"
        | "edit.deleteRelink"
        | "edit.duplicate"
        | "edit.duplicateWithoutLinks"
        | "graph.selectUpstream"
        | "graph.selectDownstream" => !editor.selection.is_empty(),
        "graph.node.disable" => false,
        _ => return None,
    })
}

pub(super) fn run(editor: &mut FunctionEditor, graph: &mut FunctionGraph, command: &str, snap: bool) -> bool {
    match command {
        "edit.selectAll" => editor.selection = graph.nodes.iter().map(|node| node.id).collect(),
        "edit.selectNone" => editor.selection.clear(),
        "edit.copy" | "edit.cut" => {
            editor.clipboard = graph.nodes.iter().filter(|node| editor.selection.contains(&node.id)).cloned().collect();
            if command == "edit.cut" {
                return remove(editor, graph, false);
            }
        }
        "edit.delete" | "edit.deleteRelink" => return remove(editor, graph, command == "edit.deleteRelink"),
        "edit.duplicate" | "edit.duplicateWithoutLinks" | "edit.paste" | "edit.pasteWithoutLinks" => {
            let duplicate = command.starts_with("edit.duplicate");
            let nodes: Vec<FunctionNode> =
                if duplicate { graph.nodes.iter().filter(|node| editor.selection.contains(&node.id)).cloned().collect() } else { editor.clipboard.clone() };
            let anchor = nodes.first().map_or([0.0; 2], |node| node.pos);
            let offset = if duplicate { [32.0; 2] } else { [editor.menu_at.x - anchor[0], editor.menu_at.y - anchor[1]] };
            let mut ids = BTreeMap::new();
            for mut node in nodes {
                let source = node.id;
                node.id = graph.next_id();
                node.pos = materialcraft_engine::graph_grid::snap_position([node.pos[0] + offset[0], node.pos[1] + offset[1]], snap);
                ids.insert(source, node.id);
                graph.nodes.push(node);
            }
            editor.selection = ids.values().copied().collect();
            for node in graph.nodes.iter_mut().filter(|node| editor.selection.contains(&node.id)) {
                for input in &mut node.inputs {
                    *input = input.and_then(|id| ids.get(&id).copied().or_else(|| (!command.ends_with("WithoutLinks")).then_some(id)));
                }
            }
            return !ids.is_empty();
        }
        "graph.selectUpstream" | "graph.selectDownstream" => {
            let upstream = command == "graph.selectUpstream";
            let mut visited = BTreeSet::new();
            let mut pending = editor.selection.clone();
            while let Some(id) = pending.pop() {
                if !visited.insert(id) {
                    continue;
                }
                if upstream {
                    if let Some(node) = graph.node(id) {
                        pending.extend(node.inputs.iter().flatten());
                    }
                } else {
                    pending.extend(graph.nodes.iter().filter(|node| node.inputs.contains(&Some(id))).map(|node| node.id));
                }
            }
            editor.selection = visited.into_iter().collect();
        }
        _ => {}
    }
    false
}

fn remove(editor: &mut FunctionEditor, graph: &mut FunctionGraph, relink: bool) -> bool {
    let replacements: BTreeMap<_, _> = graph
        .nodes
        .iter()
        .filter(|node| editor.selection.contains(&node.id))
        .map(|node| (node.id, if relink { node.inputs.first().copied().flatten() } else { None }))
        .collect();
    graph.nodes.retain(|node| !editor.selection.contains(&node.id));
    for node in &mut graph.nodes {
        for input in &mut node.inputs {
            let mut seen = BTreeSet::new();
            while let Some(id) = *input {
                let Some(replacement) = replacements.get(&id) else { break };
                if !seen.insert(id) {
                    *input = None;
                    break;
                }
                *input = *replacement;
            }
        }
    }
    if graph.output.is_some_and(|id| editor.selection.contains(&id)) {
        graph.output = None;
    }
    editor.selection.clear();
    !replacements.is_empty()
}
