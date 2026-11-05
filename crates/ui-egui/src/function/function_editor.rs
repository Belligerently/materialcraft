use egui::{Pos2, Vec2, vec2};
use materialcraft_engine::core::NodeId;

use super::{FunctionDrag, FunctionTarget};

pub struct FunctionEditor {
    pub node: Option<NodeId>,
    pub(crate) selection: Vec<u32>,
    pub(crate) inspected_node: Option<u32>,
    pub(super) clipboard: Vec<materialcraft_engine::core::FunctionNode>,
    pub target: FunctionTarget,
    pub(super) pan: Vec2,
    pub(super) zoom: f32,
    pub(super) drag: Option<FunctionDrag>,
    pub(super) menu_at: Pos2,
    pub(super) framed: bool,
}

impl FunctionEditor {
    pub fn standalone(graph: usize) -> Self {
        Self {
            node: None,
            target: FunctionTarget::Graph(graph),
            selection: Vec::new(),
            inspected_node: None,
            clipboard: Vec::new(),
            pan: vec2(-40.0, -40.0),
            zoom: 1.0,
            drag: None,
            menu_at: Pos2::ZERO,
            framed: false,
        }
    }

    pub fn new(node: NodeId, target: FunctionTarget) -> FunctionEditor {
        FunctionEditor {
            node: Some(node),
            selection: Vec::new(),
            inspected_node: None,
            clipboard: Vec::new(),
            target,
            pan: vec2(-40.0, -40.0),
            zoom: 1.0,
            drag: None,
            menu_at: Pos2::ZERO,
            framed: false,
        }
    }
}
