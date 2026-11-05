use egui::Pos2;
use materialcraft_engine::core::NodeId;
use materialcraft_engine::nodes::PortKind;

/// One port as drawn this frame
#[derive(Clone, Debug)]
pub struct PortHit {
    pub node: NodeId,
    pub output: bool,
    pub port: String,
    pub label: String,
    pub pos: Pos2,
    pub kind: PortKind,
}
