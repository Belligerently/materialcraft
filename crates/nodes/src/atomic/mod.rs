mod bitmap;
mod blend;
mod blur;
mod channels_shuffle;
mod curve;
mod directional_blur;
mod directional_warp;
mod distance;
mod dot;
mod emboss;
mod gradient_dynamic;
mod gradient_map;
mod grayscale_conversion;
mod hsl;
mod input_color;
mod input_grayscale;
mod input_value;
mod levels;
mod motion_blur;
mod normal;
mod output;
mod pattern_map;
mod pixel_processor;
mod sharpen;
mod svg;
mod text;
mod transformation;
mod uniform;
mod value_processor;
mod warp;

mod catalog;
mod direction;

pub(crate) use catalog::defs;
use direction::direction;

pub const ATOMIC: &str = "Atomic nodes";
pub const GRAPH_ITEMS: &str = "Graph items";
pub const USAGES: &[&str] = &["Base color", "Normal", "Roughness", "Metallic", "Height", "Ambient occlusion", "Emissive", "Opacity", "Specular", "Any"];
pub const USAGE_IDS: &[&str] = &["baseColor", "normal", "roughness", "metallic", "height", "ambientOcclusion", "emissive", "opacity", "specular", "any"];

pub use levels::levels_fn;
pub use normal::height_to_normal;
pub use output::output_kind;
