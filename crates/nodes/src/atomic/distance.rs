use super::ATOMIC;
use crate::ops;
use crate::{CookContext, CookResult, NodeDefinition, node};
use materialcraft_image::Image;

pub(super) fn definition() -> NodeDefinition {
    node("distance", "Distance", ATOMIC)
        .desc("Grows white areas of a mask into a distance gradient; with a source, spreads its colors outward.")
        .atomic()
        .gray_in("mask", "Mask input")
        .any_in("source", "Source input")
        .float("distance", "Maximum distance", 10.0, 0.0, 256.0)
        .boolean("combine", "Combine distance with source", false)
        .any_out()
        .cook(distance)
}

fn distance(context: &CookContext) -> CookResult {
    let mask = context.gray_or(0, 0.0);
    let maxd = (context.f("distance").max(0.0) * context.w as f32 / 256.0).max(1e-3);
    let data = mask.data.clone();
    let d2 = ops::distance_sq(context.w, context.h, |i| data.get(i).copied().unwrap_or(0.0) > 0.5, context.tile);
    let Some(src) = context.sized(1) else {
        return Ok(vec![ops::gray_from(context.w, context.h, d2.iter().map(|d| (1.0 - d.sqrt() / maxd).clamp(0.0, 1.0)).collect())]);
    };
    // Spread the source's colors: each pixel takes the color of the nearest mask pixel, found by walking down the distance field's gradient
    let dist: Vec<f32> = d2.iter().map(|d| d.sqrt()).collect();
    let combine = context.b("combine");
    let (w, h) = (context.w as i64, context.h as i64);
    let field = ops::gray_from(context.w, context.h, dist);
    Ok(vec![Image::generate(context.w, context.h, src.kind, |x, y| {
        let (mut px, mut py) = (x as f32, y as f32);
        for _ in 0..64 {
            let d = field.get_gray(px.round() as i64, py.round() as i64, context.tile);
            if d < 0.5 {
                break;
            }
            let (gx, gy) = ops::slope(&field, px.round() as i64, py.round() as i64, context.tile);
            let g = (gx * gx + gy * gy).sqrt().max(1e-4);
            px -= gx / g * d.min(8.0);
            py -= gy / g * d.min(8.0);
            if !context.tile && (px < 0.0 || py < 0.0 || px >= w as f32 || py >= h as f32) {
                break;
            }
        }
        let mut c = src.get(px.round() as i64, py.round() as i64, context.tile);
        let d = field.get_gray(x as i64, y as i64, context.tile);
        if d > maxd {
            c = [0.0, 0.0, 0.0, if src.is_gray() { 1.0 } else { 0.0 }];
        } else if combine {
            let f = (1.0 - d / maxd).clamp(0.0, 1.0);
            c = [c[0] * f, c[1] * f, c[2] * f, c[3]];
        }
        c
    })])
}
