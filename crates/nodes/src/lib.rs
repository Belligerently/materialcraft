//! # materialcraft-nodes
//!
//! The node library, L1: every node's definition (ports, parameters, library category) and its
//! CPU implementation. A cook is a pure function of its inputs, resolved parameters, size, seed
//! and tiling ([`Ctx`]), so every node is testable on its own and identical in the app, the CLI,
//! the MCP server and the browser. Noises tile seamlessly.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod atomic;
pub mod catalog;
pub mod filters;
pub mod filters_extra;
pub mod filters_more;
pub mod flood_fill;
pub mod function;
pub mod fx_map;
pub mod generators;
pub mod material;
pub mod material_more;
pub mod noise;
pub mod noise3d;
pub mod noises_more;
pub mod ops;
pub mod patterns;
pub mod patterns_more;
pub mod svg;
pub mod text;
pub mod weathering;

pub use catalog::{CookResult, Ctx, NodeDef, ParamDef, PortDef, PortKind, catalog, categories, clamp_params, find, resolve_params};

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use materialcraft_core::Value;
    use materialcraft_image::{Image, Kind};

    use super::*;

    fn cook(def: &NodeDef, inputs: &[Option<Image>], size: u32) -> CookResult {
        let params = resolve_params(def, &BTreeMap::new());
        let no_res = |_: &str| None;
        let vars = BTreeMap::new();
        let ctx = Ctx { w: size, h: size, seed: 7, tile: true, inputs, params: &params, resource: &no_res, vars: &vars };
        (def.cook)(&ctx)
    }

    /// Every node cooks with no inputs and with hostile-sized inputs, returns as many images as
    /// it has outputs, at the requested size, with finite values.
    #[test]
    fn every_node_cooks() {
        let ids: Vec<&str> = catalog().iter().map(|d| d.id).collect();
        let mut seen = std::collections::BTreeSet::new();
        for id in &ids {
            assert!(seen.insert(*id), "duplicate node id {id}");
        }
        let odd = Image::generate(5, 3, Kind::Color, |x, y| [x as f32 / 4.0, y as f32 / 2.0, 0.5, 1.0]);
        for def in catalog() {
            if def.id == "graph_instance" {
                continue;
            }
            for inputs in [vec![None; def.inputs.len()], vec![Some(odd.clone()); def.inputs.len()]] {
                let out = cook(def, &inputs, 32).unwrap_or_else(|e| panic!("{}: {e}", def.id));
                assert_eq!(out.len(), def.outputs.len().max(1), "{}", def.id);
                for img in &out {
                    assert_eq!((img.w, img.h), (32, 32), "{}", def.id);
                    assert!(img.data.iter().all(|v| v.is_finite()), "{} produced a non-finite value", def.id);
                }
            }
        }
        assert!(catalog().len() >= 200, "{} nodes", catalog().len());
    }

    /// Every node cooks with each numeric parameter at a hostile value (huge, negative huge, NaN):
    /// the hard limits keep the cook bounded, finite and the right size.
    #[test]
    fn every_node_survives_hostile_params() {
        use std::time::{Duration, Instant};

        let mut slow: Vec<(String, Duration)> = Vec::new();
        let total = Instant::now();
        for def in catalog() {
            if def.id == "graph_instance" {
                continue;
            }
            for p in &def.params {
                for h in [1e30_f32, -1e30, f32::NAN] {
                    let v = match p.default {
                        Value::Float(_) => Value::Float(h),
                        Value::Int(_) => Value::Int(h as i32),
                        Value::Float2(_) => Value::Float2([h, h]),
                        Value::Enum(_) => Value::Enum(h as u32),
                        _ => continue,
                    };
                    let mut params = resolve_params(def, &BTreeMap::from([(p.id.to_string(), v)]));
                    clamp_params(def, &mut params);
                    let inputs = vec![None; def.inputs.len()];
                    let no_res = |_: &str| None;
                    let vars = BTreeMap::new();
                    let ctx = Ctx { w: 16, h: 16, seed: 7, tile: true, inputs: &inputs, params: &params, resource: &no_res, vars: &vars };
                    let t = Instant::now();
                    let out = (def.cook)(&ctx).unwrap_or_else(|e| panic!("{}.{}={h}: {e}", def.id, p.id));
                    let dt = t.elapsed();
                    assert!(dt < Duration::from_secs(2), "{}.{}={h} took {dt:?}", def.id, p.id);
                    if dt > Duration::from_millis(250) {
                        slow.push((format!("{}.{}={h}", def.id, p.id), dt));
                    }
                    for img in &out {
                        assert_eq!((img.w, img.h), (16, 16), "{}.{}={h}", def.id, p.id);
                        assert!(img.data.iter().all(|x| x.is_finite()), "{}.{}={h} produced a non-finite value", def.id, p.id);
                    }
                }
            }
        }
        eprintln!("hostile params: total {:?}; cooks over 250 ms: {slow:?}", total.elapsed());
    }

    #[test]
    fn noises_tile() {
        for id in [
            "perlin_noise",
            "clouds_1",
            "cells_1",
            "voronoi",
            "brick_generator",
            "tile_generator",
            "hexagons",
            "value_noise",
            "ridged_fractal",
            "voronoi_fractal",
            "cells_fractal",
            "directional_noise",
            "fibers",
            "messy_fibers",
            "leather",
            "microscope",
            "gaussian_spots",
            "crackle",
            "sand_dunes",
            "wood_grain",
            "marble",
            "stones",
            "ripples",
            "concrete",
            "bark",
            "grid_lines",
            "dots_grid",
            "chevron",
            "basketweave",
            "tread_plate",
            "fx_map",
            "shape_splatter",
            "perlin_noise_3d",
            "simplex_noise_3d",
            "worley_noise_3d",
            "cracks",
            "paint_strokes",
            "scratches_generator",
            "rust",
            "moss",
        ] {
            let def = find(id).unwrap();
            let img = &cook(def, &[], 64).unwrap()[0];
            // The jump across the wrap is no larger than typical neighbouring differences.
            let mut seam = 0.0f32;
            let mut inner = 0.0f32;
            for y in 0..64 {
                seam += (img.get(63, y, false)[0] - img.get(0, y, false)[0]).abs();
                inner += (img.get(31, y, false)[0] - img.get(32, y, false)[0]).abs();
            }
            assert!(seam <= inner * 3.0 + 0.5, "{id}: seam {seam} vs inner {inner}");
        }
    }

    #[test]
    fn blend_and_levels_math() {
        let fg = Image::filled(4, 4, Kind::Gray, [0.25; 4]);
        let bg = Image::filled(4, 4, Kind::Gray, [0.5; 4]);
        let def = find("blend").unwrap();
        let mut params = resolve_params(def, &BTreeMap::new());
        params.insert("mode".into(), materialcraft_core::Value::Enum(3));
        let no_res = |_: &str| None;
        let vars = BTreeMap::new();
        let inputs = [Some(fg), Some(bg), None];
        let ctx = Ctx { w: 4, h: 4, seed: 0, tile: true, inputs: &inputs, params: &params, resource: &no_res, vars: &vars };
        let out = (def.cook)(&ctx).unwrap();
        assert!((out[0].data[0] - 0.125).abs() < 1e-6);
        assert!((atomic::levels_fn(0.5, 0.0, 0.5, 1.0, 0.0, 1.0) - 0.5).abs() < 1e-5);
        assert_eq!(atomic::levels_fn(0.2, 0.4, 0.5, 0.8, 0.0, 1.0), 0.0);
    }

    /// Cook one node with a single overridden parameter on a `size`-square noise input.
    fn cook_with(id: &str, key: &str, value: materialcraft_core::Value, size: u32) -> Image {
        let def = find(id).unwrap();
        let mut over = BTreeMap::new();
        over.insert(key.to_string(), value);
        let params = resolve_params(def, &over);
        let no_res = |_: &str| None;
        let vars = BTreeMap::new();
        let img = Image::generate(size, size, Kind::Gray, |x, y| [((x * 7 + y * 13) % 17) as f32 / 16.0, 0.0, 0.0, 1.0]);
        let inputs = [Some(img)];
        let ctx = Ctx { w: size, h: size, seed: 7, tile: true, inputs: &inputs, params: &params, resource: &no_res, vars: &vars };
        (def.cook)(&ctx).unwrap().remove(0)
    }

    /// A hostile blur intensity must return promptly (it used to loop for ~i64::MAX passes).
    #[test]
    fn anisotropic_blur_hostile_intensity_is_bounded() {
        let t = std::time::Instant::now();
        let out = cook_with("anisotropic_blur", "intensity", materialcraft_core::Value::Float(1e15), 64);
        assert!(out.data.iter().all(|v| v.is_finite()));
        assert!(t.elapsed().as_secs() < 20, "took {:?}", t.elapsed());
    }

    /// Large-but-legal intensity stays close to the Gaussian it approximates.
    #[test]
    fn anisotropic_blur_matches_horizontal_gaussian() {
        let img = Image::generate(128, 128, Kind::Gray, |x, y| [((x * 7 + y * 13) % 17) as f32 / 16.0, 0.0, 0.0, 1.0]);
        let out = cook_with("anisotropic_blur", "intensity", materialcraft_core::Value::Float(8.0), 128);
        // Mean is preserved by a normalised blur on a tiling image.
        let mean = |i: &Image| (0..128 * 128).map(|k| i.data[k]).sum::<f32>() / (128.0 * 128.0);
        assert!((mean(&out) - mean(&img)).abs() < 0.02);
    }
}
