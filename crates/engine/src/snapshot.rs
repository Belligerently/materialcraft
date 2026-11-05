use crate::AnnotationSelection;
use materialcraft_core::{NodeId, Package};

#[derive(Clone)]
pub(crate) struct Snapshot {
    pub(crate) pkg: Package,
    pub(crate) graph: usize,
    pub(crate) selection: Vec<NodeId>,
    pub(crate) annotation_selection: Option<AnnotationSelection>,
    pub(crate) label: String,
}
