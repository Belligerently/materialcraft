use materialcraft_engine::core::PatternGraph;
use std::collections::{BTreeMap, BTreeSet};

use super::PatternEditor;

pub(super) fn enabled(editor: &PatternEditor, command: &str) -> Option<bool> {
    Some(match command {
        "edit.selectAll" | "edit.selectNone" => true,
        "edit.paste" | "edit.pasteWithoutLinks" => !editor.clipboard.is_empty(),
        "edit.copy" | "edit.cut" | "edit.delete" | "edit.duplicate" | "edit.duplicateWithoutLinks" | "graph.selectUpstream" | "graph.selectDownstream" => {
            !editor.selection.is_empty()
        }
        _ => return None,
    })
}

pub(super) fn run(editor: &mut PatternEditor, graph: &mut PatternGraph, command: &str, snap: bool) -> Result<bool, String> {
    match command {
        "edit.selectAll" => editor.selection = graph.nodes.iter().map(|node| node.id).collect(),
        "edit.selectNone" => editor.selection.clear(),
        "edit.copy" | "edit.cut" => {
            editor.clipboard = graph.nodes.iter().filter(|node| editor.selection.contains(&node.id)).cloned().collect();
            if command == "edit.cut" {
                return Ok(remove(editor, graph));
            }
        }
        "edit.delete" => return Ok(remove(editor, graph)),
        "edit.duplicate" | "edit.duplicateWithoutLinks" | "edit.paste" | "edit.pasteWithoutLinks" => {
            let duplicate = command.starts_with("edit.duplicate");
            let nodes =
                if duplicate { graph.nodes.iter().filter(|node| editor.selection.contains(&node.id)).cloned().collect() } else { editor.clipboard.clone() };
            let anchor = nodes.first().map_or([0.0; 2], |node| node.position);
            let offset = if duplicate { [32.0; 2] } else { [editor.menu_at.x - anchor[0], editor.menu_at.y - anchor[1]] };
            let present: BTreeSet<_> = graph.nodes.iter().map(|node| node.id).collect();
            let mut ids = BTreeMap::new();
            for mut node in nodes {
                let original = node.id;
                node.id = graph.add(node.kind)?;
                node.position = materialcraft_engine::graph_grid::snap_position([node.position[0] + offset[0], node.position[1] + offset[1]], snap);
                ids.insert(original, node.id);
                if let Some(destination) = graph.nodes.last_mut() {
                    *destination = node;
                }
            }
            editor.selection = ids.values().copied().collect();
            for node in graph.nodes.iter_mut().filter(|node| editor.selection.contains(&node.id)) {
                for child in &mut node.children {
                    *child = child.and_then(|id| ids.get(&id).copied().or_else(|| (!command.ends_with("WithoutLinks") && present.contains(&id)).then_some(id)));
                }
            }
            return Ok(!ids.is_empty());
        }
        "graph.selectUpstream" | "graph.selectDownstream" => {
            let mut visited = BTreeSet::new();
            let mut pending = editor.selection.clone();
            while let Some(id) = pending.pop() {
                if !visited.insert(id) {
                    continue;
                }
                if command == "graph.selectDownstream" {
                    if let Some(node) = graph.nodes.iter().find(|node| node.id == id) {
                        pending.extend(node.children.iter().flatten());
                    }
                } else {
                    pending.extend(graph.nodes.iter().filter(|node| node.children.contains(&Some(id))).map(|node| node.id));
                }
            }
            editor.selection = visited.into_iter().collect();
        }
        _ => {}
    }
    Ok(false)
}

fn remove(editor: &mut PatternEditor, graph: &mut PatternGraph) -> bool {
    let before = graph.nodes.len();
    graph.nodes.retain(|node| !editor.selection.contains(&node.id));
    for child in graph.nodes.iter_mut().flat_map(|node| &mut node.children) {
        if child.is_some_and(|id| editor.selection.contains(&id)) {
            *child = None;
        }
    }
    if graph.root.is_some_and(|id| editor.selection.contains(&id)) {
        graph.root = None;
    }
    editor.selection.clear();
    graph.nodes.len() != before
}
