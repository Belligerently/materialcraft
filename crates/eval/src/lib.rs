#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod cache;
mod cache_key;
mod cooked;
mod graph_execution;
mod node_output;
mod node_result;
mod node_size;
mod options;
mod port;
mod value_input_label;

pub use cache::Cache;
pub use cooked::Cooked;
pub use graph_execution::{cook, error_image, now_ms, outputs};
pub use node_output::NodeOutput;
pub use node_result::NodeResult;
pub use node_size::node_size;
pub use options::Options;
pub use port::{Port, identifier, instance_graph, ports};
pub use value_input_label::value_input_label;

pub const MAX_DEPTH: u32 = 8;

#[cfg(test)]
mod tests;
