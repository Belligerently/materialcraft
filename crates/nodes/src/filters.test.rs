//! Round-trip test for Height to Normal -> Normal to Height on a smooth analytic bump.

use std::collections::BTreeMap;

use materialcraft_core::Value;
use materialcraft_image::{Image, Kind};

use crate::{Ctx, find, resolve_params};

fn run(id: &str, input: &Image, size: u32, over: &[(&str, Value)]) -> Image {
    let def = find(id).unwrap();
    let over: BTreeMap<String, Value> = over.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
    let params = resolve_params(def, &over);
    let no_res = |_: &str| None;
    let vars = BTreeMap::new();
    let inputs = [Some(input.clone())];
    let ctx = Ctx { w: size, h: size, seed: 1, tile: true, inputs: &inputs, params: &params, resource: &no_res, vars: &vars };
    (def.cook)(&ctx).unwrap().remove(0)
}

fn pearson(a: &Image, b: &Image) -> f32 {
    let n = (a.w * a.h) as usize;
    let (ma, mb) = (a.data.iter().take(n).sum::<f32>() / n as f32, b.data.iter().take(n).sum::<f32>() / n as f32);
    let (mut sab, mut saa, mut sbb) = (0.0f32, 0.0f32, 0.0f32);
    for i in 0..n {
        let (da, db) = (a.data[i] - ma, b.data[i] - mb);
        sab += da * db;
        saa += da * da;
        sbb += db * db;
    }
    sab / (saa * sbb).sqrt().max(1e-9)
}

#[test]
fn normal_to_height_recovers_the_broad_shape_of_a_bump() {
    for size in [256u32, 1024] {
        // A wide wrap-around Gaussian bump: nearly all of its energy is at coarse scales.
        let bump = Image::generate_uv(size, size, Kind::Gray, |u, v| {
            let (dx, dy) = (u - 0.5, v - 0.5);
            [(-(dx * dx + dy * dy) / (2.0 * 0.12 * 0.12)).exp(), 0.0, 0.0, 0.0]
        });
        let normal = run("height_to_normal", &bump, size, &[("intensity", Value::Float(2.0))]);
        let back = run("normal_to_height", &normal, size, &[]);
        let c = pearson(&bump, &back);
        assert!(c > 0.99, "size {size}: correlation {c}");
    }
}
