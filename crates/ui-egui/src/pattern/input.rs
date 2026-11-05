use egui::{PointerButton, Rect, Response, Ui};
use materialcraft_engine::core::PatternGraph;

use super::{PatternDrag, PatternEditor};
use crate::{MaterialCraftApp, graph::GraphCamera};

pub(super) fn handle(
    app: &mut MaterialCraftApp,
    editor: &mut PatternEditor,
    graph: &mut PatternGraph,
    ui: &Ui,
    response: &Response,
    camera: &GraphCamera,
) -> bool {
    let Some(pointer) = ui.input(|input| input.pointer.interact_pos()) else { return false };
    let position = camera.to_graph(pointer);
    let port = app
        .graph
        .ports
        .iter()
        .filter(|port| port.pos.distance(pointer) < (6.0 * editor.zoom).clamp(1.5, 8.0))
        .min_by(|left, right| left.pos.distance_sq(pointer).total_cmp(&right.pos.distance_sq(pointer)))
        .map(|port| {
            (
                port.node.0 as u32,
                port.output
                    .then(|| port.port.strip_prefix("output").and_then(|slot| if slot.is_empty() { Some(0) } else { slot.parse::<usize>().ok() }))
                    .flatten(),
            )
        });
    let node = app.graph.rects.iter().rev().find(|(_, bounds)| bounds.contains(pointer)).map(|(id, _)| id.0 as u32);
    let modifiers = ui.input(|input| input.modifiers);
    let mut changed = false;

    if response.hovered() && ui.input(|input| input.pointer.button_pressed(PointerButton::Primary)) {
        if let Some((id, slot)) = port {
            if modifiers.alt {
                disconnect(graph, id, slot);
                changed = true;
            } else {
                editor.drag = Some(PatternDrag::Wire { node: id, slot });
            }
        } else if let Some(id) = node {
            crate::graph::node_selection::select(&mut editor.selection, Some(id), modifiers);
            if editor.selection.contains(&id) {
                editor.drag = Some(PatternDrag::Nodes { start: position, delta: egui::Vec2::ZERO });
            }
        } else {
            let additive = modifiers.shift || modifiers.ctrl || modifiers.command;
            crate::graph::node_selection::select(&mut editor.selection, None, modifiers);
            editor.drag = Some(PatternDrag::Box { start: pointer, additive });
        }
    }
    if response.drag_started_by(PointerButton::Middle) {
        editor.drag = Some(PatternDrag::Pan);
    }
    match &mut editor.drag {
        Some(PatternDrag::Nodes { start, delta }) => *delta = position - *start,
        Some(PatternDrag::Pan) => editor.pan -= ui.input(|input| input.pointer.delta()) / editor.zoom,
        Some(PatternDrag::Box { start, .. }) => {
            ui.painter().rect_stroke(
                Rect::from_two_pos(*start, pointer),
                0.0,
                egui::Stroke::new(1.0, crate::theme::Tokens::get(ui.ctx()).accent),
                egui::StrokeKind::Inside,
            );
        }
        _ => {}
    }

    if ui.input(|input| input.pointer.button_released(PointerButton::Primary)) {
        match editor.drag.take() {
            Some(PatternDrag::Nodes { delta, .. }) if delta.length_sq() > 0.01 => {
                for node in graph.nodes.iter_mut().filter(|node| editor.selection.contains(&node.id)) {
                    node.position =
                        materialcraft_engine::graph_grid::snap_position([node.position[0] + delta.x, node.position[1] + delta.y], app.ui.prefs.snap_to_grid);
                }
                changed = true;
            }
            Some(PatternDrag::Wire { node, slot }) => {
                if let Some((target, target_slot)) = port {
                    let connection = match (slot, target_slot) {
                        (Some(slot), None) => Some((node, slot, target)),
                        (None, Some(slot)) => Some((target, slot, node)),
                        _ => None,
                    };
                    if let Some((parent, slot, child)) = connection
                        && let Some(destination) = graph.nodes.iter_mut().find(|node| node.id == parent).and_then(|node| node.children.get_mut(slot))
                    {
                        *destination = Some(child);
                        changed = true;
                    }
                }
            }
            Some(PatternDrag::Box { start, additive }) => {
                if !additive {
                    editor.selection.clear();
                }
                let bounds = Rect::from_two_pos(start, pointer);
                for (id, rectangle) in &app.graph.rects {
                    let id = id.0 as u32;
                    if bounds.intersects(*rectangle) && !editor.selection.contains(&id) {
                        editor.selection.push(id);
                    }
                }
            }
            _ => {}
        }
    }
    if ui.input(|input| input.pointer.button_released(PointerButton::Middle)) {
        editor.drag = None;
    }
    if response.secondary_clicked() {
        editor.menu_at = position;
        if let Some(id) = node
            && !editor.selection.contains(&id)
        {
            crate::graph::node_selection::select(&mut editor.selection, Some(id), egui::Modifiers::NONE);
        }
    }
    if response.hovered() && ui.input(|input| input.key_pressed(egui::Key::Space)) {
        editor.menu_at = position;
        egui::Popup::open_id(ui.ctx(), response.id.with("menu"));
    }
    changed
}

fn disconnect(graph: &mut PatternGraph, id: u32, slot: Option<usize>) {
    if let Some(slot) = slot {
        if let Some(child) = graph.nodes.iter_mut().find(|node| node.id == id).and_then(|node| node.children.get_mut(slot)) {
            *child = None;
        }
    } else {
        for child in graph.nodes.iter_mut().flat_map(|node| &mut node.children) {
            if *child == Some(id) {
                *child = None;
            }
        }
    }
}
