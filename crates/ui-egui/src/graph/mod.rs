pub(crate) mod actions;
pub(crate) mod add_node_menu;
pub(crate) mod annotation_canvas;
pub(crate) mod annotation_input;
pub(crate) mod canvas;
mod context_menu;
mod frame_hit;
#[cfg(test)]
mod frames;
mod graph_camera;
mod graph_drag;
mod graph_view;
pub(crate) mod input_visibility;
#[cfg(test)]
mod interaction_tests;
mod library_drag;
pub(crate) mod node_card;
mod node_layout;
mod node_search;
pub(crate) mod node_selection;
mod port_hit;
pub(crate) mod port_label;

pub use canvas::{NODE_W, add_node_near_selection, delete_selected_wires, header_color, node_label, node_rect, port_color, show, swap_selected_wires};
pub(crate) use graph_camera::GraphCamera;
use graph_drag::GraphDrag;
pub use graph_view::GraphView;
pub use library_drag::LibraryDrag;
use node_search::NodeSearch;
pub use port_hit::PortHit;
pub type WireRef = (materialcraft_engine::core::NodeId, String);
