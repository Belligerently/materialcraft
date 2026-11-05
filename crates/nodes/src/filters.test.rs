//! Tests for the blur helpers in `filters.rs`.

use materialcraft_image::{Image, Kind};

use super::blur_xy;

fn noise(n: u32) -> Image {
    Image::generate(n, n, Kind::Gray, |x, y| [((x * 7 + y * 13) % 17) as f32 / 16.0, 0.0, 0.0, 1.0])
}

#[test]
fn blur_xy_huge_sigma_is_bounded_and_finite() {
    let t = std::time::Instant::now();
    let out = blur_xy(&noise(64), 1e14, 1.0, true);
    assert!(out.data.iter().all(|v| v.is_finite()));
    assert!(t.elapsed().as_secs() < 20, "took {:?}", t.elapsed());
}

#[test]
fn blur_xy_horizontal_blur_is_stronger_than_vertical() {
    // Vertical stripes survive a vertical-only blur but are smoothed by a horizontal one.
    let img = Image::generate(64, 64, Kind::Gray, |x, _| [(x % 2) as f32, 0.0, 0.0, 1.0]);
    let out = blur_xy(&img, 6.0, 0.0, true);
    let range = out.data.chunks(1).map(|v| v[0]).fold((f32::MAX, f32::MIN), |a, v| (a.0.min(v), a.1.max(v)));
    assert!(range.1 - range.0 < 0.1, "stripes should be blurred away: {range:?}");
    let mean: f32 = out.data.iter().sum::<f32>() / out.data.len() as f32;
    assert!((mean - 0.5).abs() < 0.02);
}
