use super::dot::passthrough;
use super::{GRAPH_ITEMS, USAGES};
use crate::{NodeDefinition, PortKind, node};
use materialcraft_image::{Image, Kind};

pub(super) fn definition() -> NodeDefinition {
    node("output", "Output", GRAPH_ITEMS)
        .desc("An output of the graph: a material channel (usage) or any image.")
        .atomic()
        .any_in("input", "Input")
        .text("identifier", "Identifier", "output")
        .choice("usage", "Usage", USAGES, 0)
        .cook(passthrough)
}

pub fn output_kind(def: &NodeDefinition, port: usize, inputs: &[Option<Image>]) -> Kind {
    match def.outputs.get(port).map(|p| p.kind) {
        Some(PortKind::Gray) => Kind::Gray,
        Some(PortKind::Color) => Kind::Color,
        _ => inputs.iter().flatten().next().map_or(Kind::Gray, |i| i.kind),
    }
}
