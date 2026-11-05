pub const GRID_SPACING: f32 = 32.0;

pub fn snap_position(position: [f32; 2], enabled: bool) -> [f32; 2] {
    position.map(|coordinate| {
        let coordinate = if coordinate.is_finite() { coordinate.clamp(-1e7, 1e7) } else { 0.0 };
        if enabled { (coordinate / GRID_SPACING).round() * GRID_SPACING } else { coordinate }
    })
}
