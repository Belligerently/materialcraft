#[cfg(test)]
mod session_tests {
    use crate::Session;
    use materialcraft_core::Package;

    #[test]
    fn sync_cook_bumps_the_cook_epoch() {
        let mut s = Session::new(Package::new("t"));
        let before = s.cook_epoch;
        s.cook();
        assert_ne!(s.cook_epoch, before);
    }
}

use materialcraft_core::{Package, Resource};
use materialcraft_formats::{Depth, encode};
use materialcraft_image::{Image, Kind, resource_key};

use crate::session::Session;

fn normal_map_session() -> Session {
    let mut pkg = Package::new("T");
    let png = encode(&Image::filled(4, 4, Kind::Color, [0.5, 0.5, 1.0, 1.0]), "png", Depth::U8, false).unwrap();
    pkg.resources.push(Resource { name: "n".into(), file: "n.png".into(), bytes: materialcraft_core::ResourceBytes::new(png) });
    Session::new(pkg)
}

#[test]
fn the_resource_decoder_reads_data_maps_as_stored() {
    let s = normal_map_session();
    let dec = s.resource_decoder();
    let color = dec(&resource_key("n", false)).unwrap();
    let data = dec(&resource_key("n", true)).unwrap();
    // 8-bit color is sRGB-decoded (0.5 -> ~0.214); a data map keeps 0.5.
    assert!((color.get(0, 0, false)[0] - 0.214).abs() < 0.01);
    assert!((data.get(0, 0, false)[0] - 0.5).abs() < 0.01);
    assert!((data.get(0, 0, false)[2] - 1.0).abs() < 0.01);
}
