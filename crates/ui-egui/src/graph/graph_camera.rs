use egui::{Pos2, Vec2};

pub(crate) struct GraphCamera {
    pub(crate) origin: Pos2,
    pub(crate) pan: Vec2,
    pub(crate) zoom: f32,
}

impl GraphCamera {
    pub(crate) fn to_screen(&self, p: Pos2) -> Pos2 {
        self.origin + (p.to_vec2() - self.pan) * self.zoom
    }
    pub(crate) fn to_graph(&self, s: Pos2) -> Pos2 {
        ((s - self.origin) / self.zoom + self.pan).to_pos2()
    }
}
