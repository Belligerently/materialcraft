#[cfg(test)]
mod normal_blend_tests {
    use std::collections::BTreeMap;

    use materialcraft_image::{Image, Kind};

    use crate::catalog::Ctx;
    use crate::catalog::{find, resolve_params};

    /// A base normal pointing straight away from the viewer (blue = 0) used to divide by zero.
    #[test]
    fn normal_blend_with_a_black_base_stays_finite() {
        let def = find("normal_blend").expect("normal_blend exists");
        let params = resolve_params(def, &BTreeMap::new());
        let no_res = |_: &str| None;
        let vars = BTreeMap::new();
        let black = Image::filled(8, 8, Kind::Color, [0.0, 0.0, 0.0, 1.0]);
        let bump = Image::filled(8, 8, Kind::Color, [0.7, 0.4, 0.6, 1.0]);
        let inputs = vec![Some(black), Some(bump), None];
        let ctx = Ctx { w: 8, h: 8, seed: 1, tile: true, inputs: &inputs, params: &params, resource: &no_res, vars: &vars };
        let out = (def.cook)(&ctx).expect("cooks");
        assert!(out[0].data.iter().all(|v| v.is_finite()), "NaN or Inf in the blended normal");
    }
}
