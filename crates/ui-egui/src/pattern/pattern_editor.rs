use egui::{Pos2, Vec2};
use materialcraft_engine::core::{NodeId, PatternGraph, PatternNode, PatternNodeKind, Value};
use serde_json::json;

use super::PatternDrag;
use crate::MaterialCraftApp;

pub struct PatternEditor {
    pub node: NodeId,
    pub graph: usize,
    pub(crate) selection: Vec<u32>,
    pub(crate) inspected_node: Option<u32>,
    pub(super) clipboard: Vec<PatternNode>,
    pub(super) pan: Vec2,
    pub(super) zoom: f32,
    pub(super) framed: bool,
    pub(super) drag: Option<PatternDrag>,
    pub(super) menu_at: Pos2,
}

impl PatternEditor {
    pub fn new(graph: usize, node: NodeId) -> Self {
        Self {
            node,
            graph,
            selection: Vec::new(),
            inspected_node: None,
            clipboard: Vec::new(),
            pan: Vec2::ZERO,
            zoom: 1.0,
            framed: false,
            drag: None,
            menu_at: Pos2::ZERO,
        }
    }

    pub(super) fn current(&self, app: &MaterialCraftApp) -> Option<PatternGraph> {
        if app.session.graph != self.graph {
            return None;
        }

        let node = app.session.graph().ok()?.node(self.node).ok()?;
        if node.def != "fx_map" {
            return None;
        }

        let default = materialcraft_engine::nodes::find(&node.def)?.param("pattern");
        match node.params.get("pattern").or_else(|| default.map(|parameter| &parameter.default)) {
            Some(Value::PatternGraph(graph)) => Some((**graph).clone()),
            None => Some(PatternGraph::default()),
            _ => None,
        }
    }

    pub(super) fn commit(&self, app: &mut MaterialCraftApp, graph: PatternGraph) {
        app.run("graph.pattern.set", json!({"id": self.node.0, "pattern": graph}));
    }

    pub(super) fn add(&mut self, graph: &mut PatternGraph, kind: PatternNodeKind, snap: bool) -> Result<(), String> {
        let id = graph.add(kind)?;
        if let Some(node) = graph.nodes.iter_mut().find(|node| node.id == id) {
            node.position = materialcraft_engine::graph_grid::snap_position([self.menu_at.x, self.menu_at.y], snap);
        }
        self.selection = vec![id];
        Ok(())
    }
}
