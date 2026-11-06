use egui::{Event, Key, Modifiers, OutputCommand, vec2};
use egui_kittest::{Harness, kittest::Queryable};
use materialcraft_ui_egui::{MaterialCraftApp, UiState};
use serde_json::json;

fn harness(render: bool) -> Harness<'static, MaterialCraftApp> {
    let mut app = MaterialCraftApp::headless(materialcraft_engine::new_session(), UiState::default());
    app.session.preview_size = Some(32);
    let source = app.run("graph.node.add", json!({"def": "uniform", "at": [0, 0]})).unwrap()["id"].as_u64().unwrap();
    let levels = app.run("graph.node.add", json!({"def": "levels", "connectFrom": {"node": source}})).unwrap()["id"].as_u64().unwrap();
    app.run("graph.select", json!({"ids": [source, levels]})).unwrap();
    let builder = Harness::builder().with_size(vec2(1600.0, 960.0));
    let builder = if render { builder.wgpu() } else { builder };
    let mut h = builder.build_ui_state(
        |ui, app: &mut MaterialCraftApp| {
            app.step(&ui.ctx().clone());
            app.draw(ui);
        },
        app,
    );
    h.run_steps(8);
    h
}

fn copied_text(h: &Harness<'static, MaterialCraftApp>) -> String {
    h.output()
        .platform_output
        .commands
        .iter()
        .find_map(|c| if let OutputCommand::CopyText(text) = c { Some(text.clone()) } else { None })
        .expect("node copy must populate the system clipboard so native paste generates an event")
}

fn copy_paste_cut(h: &mut Harness<'static, MaterialCraftApp>) {
    h.event(Event::Copy);
    h.step();
    assert!(h.state().session.clipboard.is_some(), "native Copy event did not copy nodes");
    let payload = copied_text(h);
    assert_eq!(serde_json::from_str::<serde_json::Value>(&payload).unwrap()["nodes"].as_array().unwrap().len(), 2);
    h.event(Event::Paste(payload));
    h.step();
    assert_eq!(h.state().session.graph().unwrap().nodes.len(), 4);
    assert_eq!(h.state().session.graph().unwrap().connections.len(), 2);
    h.event(Event::Cut);
    h.step();
    assert_eq!(h.state().session.graph().unwrap().nodes.len(), 2);
    let payload = copied_text(h);
    h.event(Event::Paste(payload));
    h.step();
    assert_eq!(h.state().session.graph().unwrap().nodes.len(), 4);
    assert_eq!(h.state().session.graph().unwrap().connections.len(), 2);
    h.state_mut().run("edit.undo", json!({})).unwrap();
    assert_eq!(h.state().session.graph().unwrap().nodes.len(), 2);
    h.state_mut().run("edit.redo", json!({})).unwrap();
    assert_eq!(h.state().session.graph().unwrap().nodes.len(), 4);
}

#[test]
fn native_clipboard_events_edit_nodes_and_preserve_links() {
    copy_paste_cut(&mut harness(false));
}

#[test]
fn menu_copy_also_populates_system_clipboard() {
    let mut h = harness(false);
    h.get_by_label("Edit").click();
    h.run_steps(4);
    h.get_by_label("Copy Ctrl+C").click();
    h.step();
    assert!(!copied_text(&h).is_empty());
}

fn chord(key: Key) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND }
}

#[test]
fn mixed_clipboard_and_key_events_do_not_paste_twice() {
    let mut h = harness(false);
    h.input_mut().events.extend([Event::ModifiersChanged(Modifiers::COMMAND), Event::Copy, chord(Key::C)]);
    h.step();
    let payload = copied_text(&h);
    h.input_mut().events.extend([Event::Paste(payload), chord(Key::V)]);
    h.step();
    assert_eq!(h.state().session.graph().unwrap().nodes.len(), 4);
}

#[test]
fn raw_command_chords_continue_to_edit_nodes() {
    let mut h = harness(false);
    h.input_mut().events.extend([Event::ModifiersChanged(Modifiers::COMMAND), chord(Key::C)]);
    h.step();
    assert!(!copied_text(&h).is_empty());
    h.input_mut().events.push(chord(Key::V));
    h.step();
    assert_eq!(h.state().session.graph().unwrap().nodes.len(), 4);
    h.input_mut().events.push(chord(Key::X));
    h.step();
    assert_eq!(h.state().session.graph().unwrap().nodes.len(), 2);
}

#[test]
fn node_context_copy_populates_system_clipboard() {
    let mut h = harness(false);
    let pos = h.state().graph.rects[0].1.center();
    h.event(Event::PointerMoved(pos));
    h.step();
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos, button: egui::PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
        h.step();
    }
    h.run_steps(4);
    h.get_by_label("Copy selection").click();
    h.step();
    assert!(!copied_text(&h).is_empty());
}

#[test]
fn clipboard_events_keep_text_editing_and_do_not_touch_nodes() {
    let mut h = harness(false);
    h.get_by_label("Node label").click();
    h.step();
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.step();
    h.event(Event::Paste("Edited label".into()));
    h.step();
    let selected = *h.state().session.selection.last().unwrap();
    assert_eq!(h.state().session.graph().unwrap().node(selected).unwrap().label, "Edited label");
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.step();
    h.event(Event::Copy);
    h.step();
    assert_eq!(copied_text(&h), "Edited label");
    assert!(h.state().session.clipboard.is_none());
    h.event(Event::Cut);
    h.step();
    assert_eq!(h.state().session.graph().unwrap().nodes.len(), 2);
    assert_eq!(h.state().session.graph().unwrap().node(selected).unwrap().label, "");
}

#[test]
#[ignore = "requires an offscreen GPU"]
fn clipboard_graph_snapshot() {
    let mut h = harness(true);
    copy_paste_cut(&mut h);
    h.run_steps(10);
    let dir = std::path::PathBuf::from(std::env::var_os("MC_SNAPSHOT_DIR").expect("MC_SNAPSHOT_DIR"));
    std::fs::create_dir_all(&dir).unwrap();
    h.render().unwrap().save(dir.join("clipboard-shortcuts.png")).unwrap();
}
