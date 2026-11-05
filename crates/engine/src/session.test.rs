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

#[cfg(test)]
mod merge_tests {
    use crate::{EngineError, Session};

    fn session() -> Session {
        crate::sample_session()
    }

    fn set(s: &mut Session, key: &str) {
        s.edit_merged(key, "Set parameter", |s| {
            s.modified = true;
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn same_key_merges_until_end_merge() {
        let mut s = session();
        let base = s.history().len();
        set(&mut s, "k");
        set(&mut s, "k");
        assert_eq!(s.history().len(), base + 1, "one drag is one entry");
        s.end_merge();
        set(&mut s, "k");
        assert_eq!(s.history().len(), base + 2, "a second drag after end_merge is a new entry");
    }

    #[test]
    fn failed_merged_edit_restores_the_session() {
        let mut s = session();
        set(&mut s, "k");
        let name = s.pkg.name.clone();
        let r: Result<(), EngineError> = s.edit_merged("k", "Set parameter", |s| {
            s.pkg.name = "half-applied".into();
            Err(EngineError::Other("boom".into()))
        });
        assert!(r.is_err());
        assert_eq!(s.pkg.name, name);
    }
}

#[test]
fn failed_merged_edits_restore_clean_flags_and_history() {
    let mut s = crate::sample_session();
    s.edit_merged("k", "Set", |_| Ok(())).unwrap();
    s.modified = false;
    s.dirty = false;
    let name = s.pkg.name.clone();
    let history = s.history();
    let result: crate::Result<()> = s.edit_merged("k", "Set", |s| {
        s.pkg.name = "partial".into();
        s.changed();
        Err(crate::EngineError::Other("failed".into()))
    });
    assert!(result.is_err());
    assert_eq!(s.pkg.name, name);
    assert_eq!(s.history(), history);
    assert!(!s.modified && !s.dirty);
}
