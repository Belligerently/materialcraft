use serde::{Deserialize, Serialize};

/// The 2D view's camera: image zoom (1 = one image pixel per point) and center offset
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageCamera {
    pub zoom: f32,
    pub offset: [f32; 2],
    /// Fit the image to the view (until the user zooms)
    pub fit: bool,
}

impl Default for ImageCamera {
    fn default() -> Self {
        ImageCamera { zoom: 1.0, offset: [0.0, 0.0], fit: true }
    }
}
