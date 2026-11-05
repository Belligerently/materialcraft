//! Function canvas edits are committed through engine commands

use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2, pos2, vec2};
use egui_phosphor::regular as icon;
use materialcraft_engine::core::Value;
use materialcraft_engine::core::{FunctionGraph, FunctionNode};
use materialcraft_engine::nodes::function::FUNCTIONS;
use serde_json::json;

use crate::MaterialCraftApp;
use crate::graph::node_card;
use crate::i18n::tr;
use crate::theme::Tokens;

use super::{FunctionDrag, FunctionEditor, FunctionTarget, port_label::port_label};

fn arity(op: &str) -> usize {
    FUNCTIONS.iter().find(|o| o.id == op).map_or(0, |o| o.inputs.len())
}

fn label(op: &str) -> &'static str {
    FUNCTIONS.iter().find(|o| o.id == op).map_or("?", |o| o.label)
}

fn category_color(op: &str) -> Color32 {
    match FUNCTIONS.iter().find(|o| o.id == op).map_or("", |o| o.category) {
        "Constant" => Color32::from_rgb(0x5c, 0x62, 0x6e),
        "Variables" => Color32::from_rgb(0x6d, 0x52, 0xb5),
        "Samplers" => Color32::from_rgb(0xa8, 0x3c, 0x34),
        "Vector" | "Cast" => Color32::from_rgb(0x3a, 0x66, 0xb0),
        "Operator" => Color32::from_rgb(0x3b, 0x7f, 0x4c),
        "Logical" | "Comparison" | "Control" => Color32::from_rgb(0xb3, 0x5a, 0x2c),
        _ => Color32::from_rgb(0x1e, 0x8a, 0xa0),
    }
}

fn visible_inputs(node: &FunctionNode) -> usize {
    let count = arity(&node.op);
    crate::graph::input_visibility::count((0..count).map(|index| node.inputs.get(index).is_some_and(Option::is_some)), 1)
}

fn node_rect(node: &FunctionNode, previews: bool) -> Rect {
    node_card::bounds(node.pos, visible_inputs(node), 1, previews, node.op == "passthrough")
}

fn in_port(node: &FunctionNode, bounds: Rect, index: usize) -> Pos2 {
    node_card::port(bounds, index, visible_inputs(node), false, node.op == "passthrough")
}

fn out_port(node: &FunctionNode, bounds: Rect) -> Pos2 {
    node_card::port(bounds, 0, 1, true, node.op == "passthrough")
}

pub(super) fn current(app: &MaterialCraftApp, editor: &FunctionEditor) -> Option<FunctionGraph> {
    if let FunctionTarget::Graph(index) = editor.target {
        return app.session.pkg.graphs.get(index)?.function.clone();
    }

    let node = app.session.graph().ok()?.node(editor.node?).ok()?;
    let value = match &editor.target {
        FunctionTarget::Graph(_) => return None,
        FunctionTarget::Dynamic(parameter) => return node.functions.get(parameter).cloned(),
        FunctionTarget::Parameter(parameter) => {
            node.params.get(parameter).or_else(|| materialcraft_engine::nodes::find(&node.def)?.param(parameter).map(|parameter| &parameter.default))?
        }
        FunctionTarget::Pattern { node: id, parameter } => {
            let Value::PatternGraph(graph) = node.params.get("pattern")? else { return None };
            graph.nodes.iter().find(|node| node.id == *id)?.parameters.get(parameter)?
        }
    };

    match value {
        Value::Function(function) => Some((**function).clone()),
        _ => None,
    }
}

pub(super) fn commit(app: &mut MaterialCraftApp, editor: &FunctionEditor, graph: &FunctionGraph) {
    match &editor.target {
        FunctionTarget::Graph(_) => {
            app.run("graph.function.set", json!({"function": graph}));
        }
        FunctionTarget::Dynamic(parameter) => {
            app.run("graph.node.function", json!({"id": editor.node.map(|node| node.0), "param": parameter, "function": graph}));
        }
        FunctionTarget::Parameter(parameter) => {
            let value = Value::Function(Box::new(graph.clone()));
            app.run("graph.node.param", json!({"id": editor.node.map(|node| node.0), "param": parameter, "value": value}));
        }
        FunctionTarget::Pattern { node: id, parameter } => {
            let Some(mut pattern) =
                app.session.graph().ok().and_then(|graph| graph.node(editor.node?).ok()).and_then(|node| node.params.get("pattern")).cloned()
            else {
                return;
            };

            if let Value::PatternGraph(pattern) = &mut pattern
                && let Some(node) = pattern.nodes.iter_mut().find(|node| node.id == *id)
            {
                node.parameters.insert(parameter.clone(), Value::Function(Box::new(graph.clone())));
            }
            app.run("graph.node.param", json!({"id": editor.node.map(|node| node.0), "param": "pattern", "value": pattern}));
        }
    }
}

/// Draw the editor window when one is open
pub fn show(app: &mut MaterialCraftApp, ctx: &egui::Context) {
    if app.function_editor.as_ref().is_some_and(|editor| matches!(editor.target, FunctionTarget::Graph(_))) {
        return;
    }

    let Some(mut e) = app.function_editor.take() else { return };
    let Some(mut g) = current(app, &e) else { return };
    let title = app.session.graph().ok().and_then(|gr| gr.node(e.node?).ok()).map(crate::graph::node_label).unwrap_or_default();
    let mut open = true;
    let mut changed = false;
    let what = if e.target.parameter() == "function" { tr("Function").to_string() } else { format!("{} {}", tr("Function"), e.target.parameter()) };
    egui::Window::new(format!("{title} - {what}"))
        .id(egui::Id::unique(("fn-editor", e.node, &e.target)))
        .open(&mut open)
        .default_size(vec2(760.0, 460.0))
        .resizable(true)
        .show(ctx, |ui| {
            let tk = Tokens::get(ui.ctx());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(tr("Right-click or Space to add operations; drag from an output dot to an input; the starred node is the result."))
                        .size(11.0)
                        .color(tk.text_dim),
                );
            });
            changed = canvas(app, &mut e, &mut g, ui);
        });
    if changed {
        commit(app, &e, &g);
    }
    if open {
        app.function_editor = Some(e);
    }
}

pub(crate) fn show_graph(app: &mut MaterialCraftApp, ui: &mut Ui) {
    let index = app.session.graph;
    let mut editor =
        app.function_editor.take().filter(|editor| editor.target == FunctionTarget::Graph(index)).unwrap_or_else(|| FunctionEditor::standalone(index));
    let Some(mut graph) = current(app, &editor) else { return };

    ui.horizontal(|ui| {
        if ui.button(tr("Frame all")).clicked() {
            editor.framed = false;
        }
        ui.checkbox(&mut app.ui.prefs.snap_to_grid, tr("Snap to grid"));
        if graph.output.is_none() {
            ui.label(tr("Set a node as the output"));
        }
    });
    if canvas(app, &mut editor, &mut graph, ui) {
        commit(app, &editor, &graph);
    }
    app.function_editor = Some(editor);
}

fn preview_rect(app: &MaterialCraftApp, editor: &FunctionEditor, node: &FunctionNode, snap: bool) -> Rect {
    let mut bounds = node_rect(node, app.ui.prefs.thumbnails);
    if let Some(FunctionDrag::Node(d, delta)) = &editor.drag
        && (*d == node.id || editor.selection.contains(&node.id))
    {
        let position = materialcraft_engine::graph_grid::snap_position([node.pos[0] + delta.x, node.pos[1] + delta.y], snap);
        bounds = bounds.translate(vec2(position[0] - node.pos[0], position[1] - node.pos[1]));
    }
    if matches!(editor.target, FunctionTarget::Graph(_)) {
        bounds = bounds.translate(crate::graph::annotation_canvas::offset(app, |contents| contents.function_nodes.contains(&node.id)));
    }
    bounds
}

fn canvas(app: &mut MaterialCraftApp, editor: &mut FunctionEditor, graph: &mut FunctionGraph, ui: &mut Ui) -> bool {
    editor.selection.retain(|id| graph.node(*id).is_some());
    editor.inspected_node = crate::graph::node_selection::inspected(&editor.selection, editor.inspected_node);
    let snap = app.ui.prefs.snap_to_grid;
    let standalone = matches!(editor.target, FunctionTarget::Graph(_));
    let tokens = Tokens::get(ui.ctx());
    let (canvas_bounds, response) = ui.allocate_exact_size(ui.available_size().max(vec2(200.0, 200.0)), Sense::click_and_drag());
    crate::widgets::name(&response, egui::Role::Canvas, "Function Graph");
    let painter = ui.painter_at(canvas_bounds);
    painter.rect_filled(canvas_bounds, 4.0, Color32::from_rgb(0x26, 0x28, 0x2c));
    if standalone {
        app.graph.view = canvas_bounds;
        app.graph.rects.clear();
        app.graph.ports.clear();
        app.graph.wires.clear();
    }

    if !editor.framed || (standalone && app.ui.frame_all) {
        if standalone {
            app.ui.frame_all = false;
        }

        editor.framed = true;
        let mut destination =
            graph.nodes.iter().map(|node| node_rect(node, app.ui.prefs.thumbnails)).fold(Rect::NOTHING, |combined, bounds| combined.union(bounds));
        if standalone && let Ok(document) = app.session.graph() {
            for frame in &document.frames {
                destination = destination.union(Rect::from_min_size(pos2(frame.rect[0], frame.rect[1] - 32.0), vec2(frame.rect[2], frame.rect[3] + 32.0)));
            }
        }
        if destination.is_finite() && destination.width() > 0.0 {
            let destination = destination.expand(30.0);
            editor.zoom = (canvas_bounds.width() / destination.width()).min(canvas_bounds.height() / destination.height()).clamp(0.3, 1.2);
            editor.pan = destination.center().to_vec2() - canvas_bounds.size() * 0.5 / editor.zoom;
        }
    }

    let to_screen = |position: Pos2, editor: &FunctionEditor| canvas_bounds.min + (position.to_vec2() - editor.pan) * editor.zoom;
    let to_graph = |source: Pos2, editor: &FunctionEditor| ((source - canvas_bounds.min) / editor.zoom + editor.pan).to_pos2();
    let mut changed = false;
    let pointer = ui.input(|input| input.pointer.hover_pos());

    // Zoom and pan
    if response.hovered() {
        let scroll = ui.input(|input| input.smooth_scroll_delta.y);
        if scroll.abs() > 0.0
            && let Some(pointer_position) = pointer
        {
            let before = to_graph(pointer_position, editor);
            editor.zoom = (editor.zoom * (scroll * 0.0025).exp()).clamp(0.3, 2.5);
            editor.pan = before.to_vec2() - (pointer_position - canvas_bounds.min) / editor.zoom;
        }
    }

    let camera = crate::graph::GraphCamera { origin: canvas_bounds.min, pan: editor.pan, zoom: editor.zoom };
    let document = standalone.then(|| app.session.graph().ok().cloned()).flatten();
    let frame_hits = document.as_ref().map(|graph| crate::graph::annotation_canvas::frames(app, ui, graph, &camera)).unwrap_or_default();
    if standalone {
        app.graph.frames = frame_hits.iter().map(|frame| (frame.id, frame.rect)).collect();
    }
    crate::graph::canvas::grid(&painter, &camera, canvas_bounds);

    // Wires
    for node in &graph.nodes {
        let bounds = preview_rect(app, editor, node, snap);
        for (input_index, source_id) in node.inputs.iter().enumerate() {
            if let Some(source) = source_id.and_then(|source| graph.node(source)) {
                let source_position = to_screen(out_port(source, preview_rect(app, editor, source, snap)), editor);
                let destination = to_screen(in_port(node, bounds, input_index), editor);
                let control_offset = ((destination.x - source_position.x).abs() * 0.5).max(24.0);
                painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
                    [source_position, source_position + vec2(control_offset, 0.0), destination - vec2(control_offset, 0.0), destination],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::new(2.0, Color32::from_gray(0xb4)),
                ));
            }
        }
    }

    // Nodes
    let font = FontId::proportional((11.0 * editor.zoom).min(16.0));
    let mut remove: Option<u32> = None;
    let mut set_output: Option<u32> = None;
    let mut ports: Vec<(u32, Option<usize>, Pos2)> = Vec::new();
    let ids: Vec<u32> = graph.nodes.iter().map(|node| node.id).collect();
    for id in ids {
        let Some(node_index) = graph.nodes.iter().position(|node| node.id == id) else { continue };

        let node = &graph.nodes[node_index];
        let bounds = preview_rect(app, editor, node, snap);
        let screen_rect = Rect::from_min_max(to_screen(bounds.min, editor), to_screen(bounds.max, editor));
        if standalone {
            app.graph.rects.push((materialcraft_engine::core::NodeId(u64::from(id)), screen_rect));
        }

        let dot = node.op == "passthrough";
        let show_labels = crate::graph::port_label::visible(ui, screen_rect);
        let is_output = graph.output == Some(id);
        let selected = editor.selection.contains(&id);
        let outline = if selected {
            tokens.accent
        } else if dot {
            Color32::from_gray(0xb4)
        } else {
            Color32::from_gray(0x10)
        };

        if dot {
            node_card::paint_dot(&painter, screen_rect, outline);
        } else {
            node_card::paint(&painter, screen_rect, editor.zoom, label(&node.op), category_color(&node.op));
            node_card::outline(&painter, screen_rect, editor.zoom, Stroke::new(if selected { 2.0 } else { 1.0 }, outline));
            let body = node_card::body(screen_rect, editor.zoom);
            if node.value.is_none() {
                painter.text(body.center(), Align2::CENTER_CENTER, icon::CODE, FontId::proportional(30.0 * editor.zoom), tokens.text_dim);
            }
            if is_output {
                let header = node_card::header(screen_rect, editor.zoom);
                painter.text(header.right_center() - vec2(4.0, 0.0), Align2::RIGHT_CENTER, icon::STAR, font.clone(), Color32::GOLD);
            }
        }

        let port_radius = (4.5 * editor.zoom).clamp(2.5, 7.0);
        let port_color = crate::graph::port_color(materialcraft_engine::nodes::PortKind::Value);
        if let Some(definition) = materialcraft_engine::nodes::function::definition(&node.op) {
            for (input_index, identifier) in definition.inputs.iter().take(visible_inputs(node)).enumerate() {
                let position = to_screen(in_port(node, bounds, input_index), editor);
                painter.circle(position, port_radius, port_color, Stroke::new(1.0, Color32::from_gray(0x18)));
                if show_labels {
                    crate::graph::port_label::paint(
                        &painter,
                        position - vec2(10.0, 0.0),
                        Align2::RIGHT_CENTER,
                        port_label(identifier),
                        editor.zoom,
                        tokens.text_strong,
                    );
                }
                ports.push((id, Some(input_index), position));
            }
        }

        let output_position = to_screen(out_port(node, bounds), editor);
        painter.circle(output_position, port_radius, port_color, Stroke::new(1.0, Color32::from_gray(0x18)));
        if show_labels {
            crate::graph::port_label::paint(
                &painter,
                output_position + vec2(10.0, 0.0),
                Align2::LEFT_CENTER,
                port_label("output"),
                editor.zoom,
                tokens.text_strong,
            );
        }
        ports.push((id, None, output_position));

        let body = ui.interact(screen_rect, ui.scope_id().with(("fn-node", id)), Sense::click_and_drag());
        crate::widgets::name(&body, egui::Role::Button, label(&graph.nodes[node_index].op));

        if !dot && node.value.is_some() {
            let content = node_card::body(screen_rect, editor.zoom).shrink(6.0 * editor.zoom);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(content));
            child.set_clip_rect(content.intersect(canvas_bounds));
            if let Some(value) = graph.nodes[node_index].value.as_mut() {
                changed |= crate::value_editor::show(&mut child, value);
            }
        }

        if (body.contains_pointer() && ui.input(|input| input.pointer.primary_pressed())) || body.secondary_clicked() {
            let modifiers = if body.secondary_clicked() { egui::Modifiers::NONE } else { ui.input(|input| input.modifiers) };
            crate::graph::node_selection::select(&mut editor.selection, Some(id), modifiers);
            if standalone && app.session.annotation_selection.is_some() {
                app.run("graph.annotation.select", json!({}));
            }
        }

        if body.drag_started() && editor.selection.contains(&id) && editor.drag.is_none() && !app.graph.is_dragging() {
            editor.drag = Some(FunctionDrag::Node(id, Vec2::ZERO));
        }

        body.context_menu(|ui| {
            changed |= super::context_menu::show(app, editor, graph, ui);
            ui.separator();
            if ui.button(format!("{}  {}", icon::STAR, tr("Set as output node"))).clicked() {
                set_output = Some(id);
                ui.close();
            }
            if ui.button(format!("{}  {}", icon::X, tr("Delete"))).clicked() {
                remove = Some(id);
                ui.close();
            }
        });
    }

    if let Some(id) = set_output {
        graph.output = Some(id);
        changed = true;
    }

    if let Some(id) = remove {
        graph.nodes.retain(|node| node.id != id);
        for node in &mut graph.nodes {
            for source in &mut node.inputs {
                if *source == Some(id) {
                    *source = None;
                }
            }
        }
        if graph.output == Some(id) {
            graph.output = None;
        }
        changed = true;
    }

    if standalone {
        app.graph.ports = ports
            .iter()
            .map(|(id, input, position)| {
                let identifier = input
                    .and_then(|slot| {
                        let node = graph.node(*id)?;
                        materialcraft_engine::nodes::function::definition(&node.op)?.inputs.get(slot).copied()
                    })
                    .unwrap_or(if input.is_some() { "input" } else { "output" });

                crate::graph::PortHit {
                    node: materialcraft_engine::core::NodeId(u64::from(*id)),
                    port: input.map_or_else(|| "output".into(), |slot| slot.to_string()),
                    label: port_label(identifier).into(),
                    output: input.is_none(),
                    kind: materialcraft_engine::nodes::PortKind::Value,
                    pos: *position,
                }
            })
            .collect();
        app.ui.graph_camera.pan = [editor.pan.x, editor.pan.y];
        app.ui.graph_camera.zoom = editor.zoom;
    }

    // Gestures: wires from output dots, panning
    let near = |pointer_position: Pos2| {
        ports
            .iter()
            .filter(|(id, _, position)| {
                let dot = graph.node(*id).is_some_and(|node| node.op == "passthrough");
                let radius = if dot { (3.5 * editor.zoom).max(2.0) } else { 9.0 };
                position.distance(pointer_position) < radius
            })
            .min_by(|first, second| first.2.distance(pointer_position).total_cmp(&second.2.distance(pointer_position)))
            .cloned()
    };
    if response.drag_started_by(egui::PointerButton::Middle) || (response.drag_started() && ui.input(|input| input.modifiers.alt)) {
        editor.drag = Some(FunctionDrag::Pan);
    } else if ui.input(|input| input.pointer.primary_pressed())
        && editor.drag.is_none()
        && !app.graph.is_dragging()
        && let Some(pointer_position) = pointer
        && canvas_bounds.contains(pointer_position)
        && let Some((id, None, _)) = near(pointer_position)
    {
        editor.drag = Some(FunctionDrag::Wire(id));
    }

    let delta = ui.input(|input| input.pointer.delta());
    match &mut editor.drag {
        Some(FunctionDrag::Node(_, offset)) => *offset += delta / editor.zoom,
        Some(FunctionDrag::Pan) => editor.pan -= delta / editor.zoom,
        Some(FunctionDrag::Wire(id)) => {
            if let (Some(pointer_position), Some(source_id)) = (pointer, ports.iter().find(|(node, source, _)| node == id && source.is_none())) {
                painter.line_segment([source_id.2, pointer_position], Stroke::new(2.0, tokens.accent));
            }
        }
        None => {}
    }

    if ui.input(|input| input.pointer.any_released()) {
        match editor.drag.take() {
            Some(FunctionDrag::Node(id, offset)) => {
                if offset.length() > 0.5 {
                    for node in graph.nodes.iter_mut().filter(|node| node.id == id || editor.selection.contains(&node.id)) {
                        node.pos = materialcraft_engine::graph_grid::snap_position([node.pos[0] + offset.x, node.pos[1] + offset.y], snap);
                    }
                    changed = true;
                }
            }
            Some(FunctionDrag::Wire(source_id)) => {
                if let Some(pointer_position) = pointer
                    && let Some((destination_id, Some(slot), _)) = near(pointer_position)
                    && destination_id != source_id
                    && !reaches(graph, destination_id, source_id)
                    && let Some(node) = graph.nodes.iter_mut().find(|node| node.id == destination_id)
                {
                    if node.inputs.len() <= slot {
                        node.inputs.resize(slot + 1, None);
                    }
                    node.inputs[slot] = Some(source_id);
                    changed = true;
                }
            }
            _ => {}
        }
    }

    if response.clicked() {
        crate::graph::node_selection::select(&mut editor.selection, None, ui.input(|input| input.modifiers));
    }

    // Adding operations
    if response.secondary_clicked()
        && let Some(pointer_position) = pointer
    {
        editor.menu_at = to_graph(pointer_position, editor);
    }
    if response.hovered()
        && ui.input(|input| input.key_pressed(egui::Key::Space))
        && let Some(pointer_position) = pointer
    {
        editor.menu_at = to_graph(pointer_position, editor);
        egui::Popup::open_id(ui.ctx(), response.id.with("menu"));
    }
    response.context_menu(|ui| {
        changed |= super::context_menu::show(app, editor, graph, ui);
    });

    if let Some(document) = &document {
        let comments = crate::graph::annotation_canvas::comments(app, ui, document, &camera);
        app.graph.comments = comments.clone();
        crate::graph::annotation_canvas::pins(app, ui, document, &camera);
        if editor.drag.is_none() {
            crate::graph::annotation_input::handle(app, ui, &camera, &frame_hits, &comments);
            if app.session.annotation_selection.is_some() {
                editor.selection.clear();
            }
        }
    }
    editor.inspected_node = crate::graph::node_selection::inspected(&editor.selection, editor.inspected_node).filter(|id| graph.node(*id).is_some());
    changed
}

// Wiring b into a is invalid when a already feeds b
fn reaches(g: &FunctionGraph, a: u32, b: u32) -> bool {
    let mut stack = vec![b];
    let mut seen = std::collections::BTreeSet::new();
    while let Some(n) = stack.pop() {
        if n == a {
            return true;
        }
        if !seen.insert(n) {
            continue;
        }
        if let Some(node) = g.node(n) {
            stack.extend(node.inputs.iter().flatten());
        }
    }
    false
}

pub(crate) fn enabled(app: &MaterialCraftApp, command: &str) -> Option<bool> {
    if app.session.annotation_selection.is_some() || !app.session.graph().is_ok_and(|graph| graph.function.is_some()) {
        return None;
    }
    super::selection::enabled(app.function_editor.as_ref()?, command)
}

pub(crate) fn invoke(app: &mut MaterialCraftApp, command: &str) -> bool {
    if enabled(app, command).is_none() {
        return false;
    }
    let Some(mut editor) = app.function_editor.take() else { return false };
    if let Some(mut graph) = current(app, &editor)
        && super::selection::run(&mut editor, &mut graph, command, app.ui.prefs.snap_to_grid)
    {
        commit(app, &editor, &graph);
    }
    app.function_editor = Some(editor);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_are_detected() {
        let mut g = FunctionGraph::default();
        let a = g.add("get_float2", &[], Some(Value::Text("$pos".into())));
        let b = g.add("swizzle1", &[a], Some(Value::Int(0)));
        // a feeds b: wiring b into a would loop
        assert!(reaches(&g, a, b));
        assert!(!reaches(&g, b, a));
        assert_eq!(arity("add"), 2);
        assert!(materialcraft_engine::nodes::function::default_value("const_float1").is_some());
    }
}
