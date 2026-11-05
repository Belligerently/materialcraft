use egui::{Color32, Rect, Sense, Stroke, Ui, vec2};
use materialcraft_engine::core::{NodeId, PatternGraph};
use materialcraft_engine::nodes::PortKind;
use std::collections::BTreeMap;

use super::{PatternDrag, PatternEditor, geometry};
use crate::{
    MaterialCraftApp,
    graph::{GraphCamera, PortHit},
    i18n::tr,
    theme::Tokens,
};

const PORT_COLOR: Color32 = Color32::from_rgb(165, 157, 215);

pub(crate) fn show(app: &mut MaterialCraftApp, ui: &mut Ui) {
    let Some(mut editor) = app.pattern_editor.take() else { return };
    let Some(mut graph) = editor.current(app) else { return };
    editor.selection.retain(|id| graph.nodes.iter().any(|node| node.id == *id));
    let parent = app.session.graph().map(|graph| graph.name.clone()).unwrap_or_default();
    let label = app.session.graph().ok().and_then(|graph| graph.node(editor.node).ok()).map(crate::graph::node_label).unwrap_or_default();
    let mut back = false;
    ui.horizontal(|ui| {
        back = ui.button(&parent).on_hover_text(tr("Return to parent graph")).clicked();
        ui.label("›");
        ui.label(label);
    });
    ui.horizontal(|ui| {
        if ui.button(tr("Frame all")).clicked() {
            editor.framed = false;
        }
        ui.checkbox(&mut app.ui.prefs.snap_to_grid, tr("Snap to grid"));
        if graph.root.is_none() {
            ui.label(tr("Set a node as the root"));
        }
    });

    editor.inspected_node = crate::graph::node_selection::inspected(&editor.selection, editor.inspected_node);
    let changed = canvas(app, &mut editor, &mut graph, ui);
    editor.inspected_node =
        crate::graph::node_selection::inspected(&editor.selection, editor.inspected_node).filter(|id| graph.nodes.iter().any(|node| node.id == *id));
    if changed {
        editor.commit(app, graph);
    }
    if back {
        app.function_editor = None;
    } else {
        app.pattern_editor = Some(editor);
    }
}

fn canvas(app: &mut MaterialCraftApp, editor: &mut PatternEditor, graph: &mut PatternGraph, ui: &mut Ui) -> bool {
    let (bounds, response) = ui.allocate_exact_size(ui.available_size().max(vec2(100.0, 100.0)), Sense::click_and_drag());
    crate::widgets::name(&response, egui::Role::Canvas, "FX-Map graph");
    let painter = ui.painter_at(bounds);
    painter.rect_filled(bounds, 0.0, Color32::from_rgb(0x26, 0x28, 0x2c));
    let snap = app.ui.prefs.snap_to_grid;
    let frame_key = response.hovered() && !ui.ctx().egui_wants_keyboard_input() && ui.input(|input| input.key_pressed(egui::Key::F));
    if !editor.framed || app.ui.frame_all || frame_key {
        let destination = graph.nodes.iter().map(|node| geometry::bounds(node, editor, snap)).fold(Rect::NOTHING, |all, node| all.union(node));
        if destination.is_finite() {
            let destination = destination.expand(48.0);
            editor.zoom = (bounds.width() / destination.width()).min(bounds.height() / destination.height()).clamp(0.15, 1.2);
            editor.pan = destination.center().to_vec2() - bounds.size() * 0.5 / editor.zoom;
        }
        editor.framed = true;
        app.ui.frame_all = false;
    }

    let mut camera = GraphCamera { origin: bounds.min, pan: editor.pan, zoom: editor.zoom };
    if response.hovered()
        && let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
    {
        let scroll = ui.input(|input| input.smooth_scroll_delta.y);
        if scroll != 0.0 {
            let before = camera.to_graph(pointer);
            editor.zoom = (editor.zoom * (scroll * 0.0025).exp()).clamp(0.15, 3.0);
            editor.pan = before.to_vec2() - (pointer - bounds.min) / editor.zoom;
            camera.pan = editor.pan;
            camera.zoom = editor.zoom;
        }
    }
    crate::graph::canvas::grid(&painter, &camera, bounds);
    app.graph.view = bounds;
    app.graph.rects.clear();
    app.graph.ports.clear();
    app.graph.wires.clear();
    app.graph.frames.clear();
    app.graph.comments.clear();

    let rectangles: BTreeMap<_, _> = graph
        .nodes
        .iter()
        .map(|node| {
            let bounds = geometry::bounds(node, editor, snap);
            (node.id, Rect::from_min_max(camera.to_screen(bounds.min), camera.to_screen(bounds.max)))
        })
        .collect();
    for node in &graph.nodes {
        let Some(bounds) = rectangles.get(&node.id) else { continue };
        for (slot, child) in node.children.iter().enumerate() {
            if let Some(child_bounds) = child.and_then(|id| rectangles.get(&id)) {
                let points = geometry::wire(geometry::port(*bounds, Some(slot), node.children.len()), child_bounds.center_top());
                painter.add(egui::epaint::CubicBezierShape::from_points_stroke(points, false, Color32::TRANSPARENT, Stroke::new(2.0, PORT_COLOR)));
                app.graph.wires.push(((NodeId(u64::from(node.id)), node.kind.child_port(slot).unwrap_or_default().to_string()), points));
            }
        }
    }

    let tokens = Tokens::get(ui.ctx());
    for node in &graph.nodes {
        let Some(bounds) = rectangles.get(&node.id).copied() else { continue };
        app.graph.rects.push((NodeId(u64::from(node.id)), bounds));
        let outline = if editor.selection.contains(&node.id) { tokens.accent } else { Color32::from_gray(30) };
        super::node_canvas::paint(&painter, bounds, node.kind, editor.zoom, outline);
        if graph.root == Some(node.id) {
            painter.text(
                bounds.right_top() + vec2(-4.0, 4.0) * editor.zoom,
                egui::Align2::RIGHT_TOP,
                egui_phosphor::regular::STAR,
                egui::FontId::proportional(11.0 * editor.zoom),
                Color32::GOLD,
            );
        }
        let node_response = ui.interact(bounds, response.id.with(("node", node.id)), Sense::hover());
        crate::widgets::name(&node_response, egui::Role::Button, node.kind.label());

        for slot in std::iter::once(None).chain((0..node.children.len()).map(Some)) {
            let position = geometry::port(bounds, slot, node.children.len());
            let label = slot.map_or("Input", |slot| geometry::port_label(node.kind, slot));
            painter.circle_filled(position, 5.0 * editor.zoom, PORT_COLOR);
            painter.circle_stroke(position, 5.0 * editor.zoom, Stroke::new(1.5, Color32::from_gray(30)));
            ui.interact(Rect::from_center_size(position, vec2(16.0, 16.0)), response.id.with((node.id, slot)), Sense::hover())
                .on_hover_text(tr("Drag to connect; Alt-click to disconnect"));
            app.graph.ports.push(PortHit {
                node: NodeId(u64::from(node.id)),
                output: slot.is_some(),
                port: slot.map_or_else(|| "input".into(), |slot| node.kind.child_port(slot).unwrap_or_default().to_string()),
                label: label.into(),
                pos: position,
                kind: PortKind::Any,
            });
        }
    }

    let mut changed = super::input::handle(app, editor, graph, ui, &response, &camera);
    if let Some(PatternDrag::Wire { node, slot }) = editor.drag
        && let Some(bounds) = rectangles.get(&node)
        && let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
    {
        let count = graph.nodes.iter().find(|candidate| candidate.id == node).map_or(0, |node| node.children.len());
        let source = geometry::port(*bounds, slot, count);
        let points = if slot.is_some() { geometry::wire(source, pointer) } else { geometry::wire(pointer, source) };
        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(points, false, Color32::TRANSPARENT, Stroke::new(2.0, PORT_COLOR)));
    }
    response.context_menu(|ui| changed |= super::context_menu::show(app, editor, graph, ui));
    changed
}
