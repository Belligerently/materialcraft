#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod commands;
pub mod edit_cmds;
pub mod file_cmds;
pub mod graph_cmds;
pub mod graph_grid;
pub mod inspect;
pub mod material_cmds;
pub mod node_cmds;
pub mod samples;
pub mod session;
mod value_input_commands;
pub mod view_cmds;
pub mod wire_cmds;

pub use commands::{CommandSpec, command_specs, execute, find};
pub use session::{CameraPrefs, Session, View2d, View3d, WARNING_PREFIX};
mod clip;
mod connection_commands;
pub use clip::Clip;

pub use materialcraft_core as core;
pub use materialcraft_eval as eval;
pub use materialcraft_formats as formats;
pub use materialcraft_image as image;
pub use materialcraft_nodes as nodes;

mod engine_error;
pub use engine_error::EngineError;

pub type Result<T> = std::result::Result<T, EngineError>;

/// Version string shown in About, `--version` and window titles
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

mod startup;
pub use startup::{new_session, sample_session};

mod atomic_sample;
mod atomic_sample_commands;
pub use atomic_sample::atomic_sample;

mod material_options;
pub use material_options::MaterialOptions;

mod exported;
pub use exported::Exported;

mod annotation_commands;
mod annotation_selection;
mod comment_commands;
mod frame_commands;
mod frame_contents;
mod function_commands;
mod node_graph_commands;
mod pattern_graph_commands;
mod snapshot;
pub use annotation_selection::AnnotationSelection;
pub use frame_contents::FrameContents;
