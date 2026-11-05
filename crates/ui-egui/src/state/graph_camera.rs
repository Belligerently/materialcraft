use serde::{Deserialize, Serialize};

/// The graph view's camera: the graph point at the view's top-left corner, and the zoom
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphCamera {
    pub pan: [f32; 2],
    pub zoom: f32,
}

impl Default for GraphCamera {
    fn default() -> Self {
        GraphCamera { pan: [-60.0, -60.0], zoom: 0.8 }
    }
}
