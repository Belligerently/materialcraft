//! # materialcraft-formats
//!
//! File formats, L2: SBS packages (`.sbs`, the main project format, see [`sbs`]), the older native `.mcraft` package (a zip holding `package.json` and the embedded
//! `resources/`), bitmap decoding for Bitmap nodes, and texture export (PNG 8/16, JPEG, TGA,
//! TIFF 8/16, BMP, WEBP, EXR 32F, HDR) with the output naming pattern, and meshes for the 3D view
//! (OBJ, glTF/GLB), and portable materials (`.mcmat`, see [`material`]). Byte-based APIs work
//! everywhere, including the browser; path helpers are native only.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod material;
pub mod mesh;
pub mod sbs;

use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use materialcraft_core::{OutputFormat, Package};
use materialcraft_image::{Image, Kind, MAX_SIDE, linear_to_srgb, srgb_to_linear};

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("not a MaterialCraft package: {0}")]
    BadPackage(String),
    #[error("image: {0}")]
    Image(String),
    #[error("I/O: {0}")]
    Io(String),
}

pub type Result<T> = std::result::Result<T, FormatError>;

/// Largest `package.json` and resource accepted when reading (allocation caps).
const MAX_JSON: u64 = 64 << 20;
const MAX_RESOURCE: u64 = 256 << 20;

/// Export file types: (extension, label).
pub const EXPORT_FORMATS: &[(&str, &str)] =
    &[("png", "PNG"), ("jpg", "JPEG"), ("tga", "TGA"), ("tif", "TIFF"), ("bmp", "BMP"), ("webp", "WEBP"), ("exr", "OpenEXR"), ("hdr", "Radiance HDR")];

/// Image file extensions [`decode`] reads.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "tga", "tif", "tiff", "bmp", "webp", "exr", "hdr"];

/// Write a package to `.mcraft` bytes.
pub fn save_package(pkg: &Package) -> Result<Vec<u8>> {
    let json = serde_json::to_vec_pretty(pkg).map_err(|e| FormatError::Io(e.to_string()))?;
    let mut buf = Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let io = |e: zip::result::ZipError| FormatError::Io(e.to_string());
        z.start_file("package.json", opts).map_err(io)?;
        z.write_all(&json).map_err(|e| FormatError::Io(e.to_string()))?;
        for r in &pkg.resources {
            // Images are already compressed.
            let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            z.start_file(format!("resources/{}", safe_file_name(&r.file)), stored).map_err(io)?;
            z.write_all(&r.bytes).map_err(|e| FormatError::Io(e.to_string()))?;
        }
        z.finish().map_err(io)?;
    }
    Ok(buf.into_inner())
}

/// Read a package from `.mcraft` bytes.
pub fn load_package(bytes: &[u8]) -> Result<Package> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| FormatError::BadPackage(e.to_string()))?;
    let mut json = Vec::new();
    z.by_name("package.json")
        .map_err(|_| FormatError::BadPackage("package.json is missing".into()))?
        .take(MAX_JSON)
        .read_to_end(&mut json)
        .map_err(|e| FormatError::Io(e.to_string()))?;
    let mut pkg: Package = serde_json::from_slice(&json).map_err(|e| FormatError::BadPackage(e.to_string()))?;
    for r in &mut pkg.resources {
        let name = format!("resources/{}", safe_file_name(&r.file));
        let mut data = Vec::new();
        if let Ok(mut f) = z.by_name(&name) {
            f.by_ref().take(MAX_RESOURCE).read_to_end(&mut data).map_err(|e| FormatError::Io(e.to_string()))?;
        }
        r.bytes = Arc::new(data);
    }
    Ok(pkg)
}

/// A file name with no path parts (resources live flat in `resources/`).
pub fn safe_file_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let cleaned: String = base.chars().map(|c| if c.is_alphanumeric() || "._- ".contains(c) { c } else { '_' }).collect();
    let cleaned = cleaned.trim_start_matches('.').to_string();
    if cleaned.is_empty() { "resource".into() } else { cleaned }
}

/// Decode image file bytes into a linear float image (8-bit color is sRGB-decoded; grayscale
/// files stay grayscale).
pub fn decode(bytes: &[u8]) -> Result<Image> {
    let img_err = |e: image::ImageError| FormatError::Image(e.to_string());
    // TGA has no signature: assume it when the format cannot be guessed.
    let open = || -> Result<image::ImageReader<Cursor<&[u8]>>> {
        let mut r = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format().map_err(|e| FormatError::Image(e.to_string()))?;
        if r.format().is_none() {
            r.set_format(image::ImageFormat::Tga);
        }
        Ok(r)
    };
    // Read the size from the header first, so an oversized image is refused before any pixels
    // are allocated or decoded.
    let (w, h) = open()?.into_dimensions().map_err(img_err)?;
    if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE {
        return Err(FormatError::Image(format!("{w}x{h} is outside 1..={MAX_SIDE} pixels")));
    }
    let mut reader = open()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    reader.limits(limits);
    let img = reader.decode().map_err(img_err)?;
    let gray = matches!(img.color(), image::ColorType::L8 | image::ColorType::L16 | image::ColorType::La8 | image::ColorType::La16);
    let float = matches!(img.color(), image::ColorType::Rgb32F | image::ColorType::Rgba32F);
    if gray {
        let l = img.into_luma16();
        let data: Vec<f32> = l.pixels().map(|p| p.0[0] as f32 / 65535.0).collect();
        return Ok(Image { w, h, kind: Kind::Gray, data: Arc::new(data) });
    }
    let rgba = img.into_rgba32f();
    let data: Vec<f32> = rgba
        .pixels()
        .flat_map(|p| {
            let [r, g, b, a] = p.0;
            if float { [r, g, b, a] } else { [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), a] }
        })
        .collect();
    Ok(Image { w, h, kind: Kind::Color, data: Arc::new(data) })
}

/// Bit depth for export.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
    U8,
    U16,
    F32,
}

impl Depth {
    pub fn from_format(f: OutputFormat) -> Depth {
        match f {
            OutputFormat::U8 => Depth::U8,
            OutputFormat::F16 | OutputFormat::F32 => Depth::F32,
            _ => Depth::U16,
        }
    }
}

/// Encode `img` as `ext` (see [`EXPORT_FORMATS`]). `srgb`: color values are written through the
/// sRGB curve (base color); data maps (normal, roughness…) are written linear.
pub fn encode(img: &Image, ext: &str, depth: Depth, srgb: bool) -> Result<Vec<u8>> {
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    let enc = |v: f32| if srgb && img.kind == Kind::Color { linear_to_srgb(v) } else { v };
    let (w, h) = (img.w, img.h);
    let color = img.kind == Kind::Color;
    let px = |i: usize| -> [f32; 4] {
        if color {
            let s = img.data.get(i * 4..i * 4 + 4).unwrap_or(&[0.0, 0.0, 0.0, 1.0]);
            [enc(s[0]), enc(s[1]), enc(s[2]), s[3]]
        } else {
            let v = img.data.get(i).copied().unwrap_or(0.0);
            [v, v, v, 1.0]
        }
    };
    let n = (w * h) as usize;
    let fmt = match ext.as_str() {
        "png" => image::ImageFormat::Png,
        "jpg" | "jpeg" => image::ImageFormat::Jpeg,
        "tga" => image::ImageFormat::Tga,
        "tif" | "tiff" => image::ImageFormat::Tiff,
        "bmp" => image::ImageFormat::Bmp,
        "webp" => image::ImageFormat::WebP,
        "exr" => image::ImageFormat::OpenExr,
        "hdr" => image::ImageFormat::Hdr,
        other => return Err(FormatError::Image(format!("unsupported export format `{other}`"))),
    };
    let float_only = matches!(fmt, image::ImageFormat::OpenExr | image::ImageFormat::Hdr);
    let sixteen_ok = matches!(fmt, image::ImageFormat::Png | image::ImageFormat::Tiff);
    let q8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    let q16 = |v: f32| (v.clamp(0.0, 1.0) * 65535.0 + 0.5) as u16;
    let dynimg: image::DynamicImage = if float_only {
        let data: Vec<f32> = (0..n)
            .flat_map(|i| {
                let p = px(i);
                [p[0], p[1], p[2]]
            })
            .collect();
        let buf = image::Rgb32FImage::from_raw(w, h, data).ok_or_else(|| FormatError::Image("bad buffer".into()))?;
        buf.into()
    } else if depth != Depth::U8 && sixteen_ok {
        if color {
            let data: Vec<u16> = (0..n).flat_map(|i| px(i).map(q16)).collect();
            image::ImageBuffer::<image::Rgba<u16>, _>::from_raw(w, h, data).ok_or_else(|| FormatError::Image("bad buffer".into()))?.into()
        } else {
            let data: Vec<u16> = (0..n).map(|i| q16(px(i)[0])).collect();
            image::ImageBuffer::<image::Luma<u16>, _>::from_raw(w, h, data).ok_or_else(|| FormatError::Image("bad buffer".into()))?.into()
        }
    } else if color && !matches!(fmt, image::ImageFormat::Jpeg) {
        let data: Vec<u8> = (0..n).flat_map(|i| px(i).map(q8)).collect();
        image::RgbaImage::from_raw(w, h, data).ok_or_else(|| FormatError::Image("bad buffer".into()))?.into()
    } else if color {
        let data: Vec<u8> = (0..n)
            .flat_map(|i| {
                let p = px(i);
                [q8(p[0]), q8(p[1]), q8(p[2])]
            })
            .collect();
        image::RgbImage::from_raw(w, h, data).ok_or_else(|| FormatError::Image("bad buffer".into()))?.into()
    } else {
        let data: Vec<u8> = (0..n).map(|i| q8(px(i)[0])).collect();
        image::GrayImage::from_raw(w, h, data).ok_or_else(|| FormatError::Image("bad buffer".into()))?.into()
    };
    let mut out = Cursor::new(Vec::new());
    dynimg.write_to(&mut out, fmt).map_err(|e| FormatError::Image(e.to_string()))?;
    Ok(out.into_inner())
}

/// The file stem of an exported output: `pattern` with `$(graph)`, `$(identifier)` and
/// `$(usage)` replaced, sanitized for file systems.
pub fn output_name(pattern: &str, graph: &str, identifier: &str, usage: &str) -> String {
    let s = pattern.replace("$(graph)", graph).replace("$(identifier)", identifier).replace("$(usage)", usage);
    safe_file_name(&s)
}

/// Write `bytes` to `path` through a temporary file and a rename, so a failed write never leaves
/// a truncated file behind.
#[cfg(not(target_arch = "wasm32"))]
pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    // A temp name unique per process and call, so two saves of one path never share a file.
    let unique = format!(".{}.{}.tmp", std::process::id(), COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_else(|| "out".into());
    name.push(unique);
    let tmp = path.with_file_name(name);
    let write = |tmp: &std::path::Path| -> std::io::Result<()> {
        let mut f = std::fs::File::create(tmp)?;
        f.write_all(bytes)?;
        // The data must be on disk before the rename publishes it.
        f.sync_all()
    };
    if let Err(e) = write(&tmp) {
        let _ = std::fs::remove_file(&tmp);
        return Err(FormatError::Io(format!("{}: {e}", tmp.display())));
    }
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        FormatError::Io(format!("{}: {e}", path.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use materialcraft_core::{Graph, Resource};

    #[test]
    fn package_round_trip() {
        let mut p = Package::new("Test");
        let mut g = Graph::new("Bricks");
        let a = g.add_node("brick_generator", [0.0, 0.0]);
        let o = g.add_node("output", [200.0, 0.0]);
        g.connect(a, "output", o, "input").unwrap();
        p.graphs.push(g);
        let png = encode(&Image::filled(4, 4, Kind::Color, [1.0, 0.5, 0.0, 1.0]), "png", Depth::U8, true).unwrap();
        p.resources.push(Resource { name: "tex".into(), file: "../../evil/tex.png".into(), bytes: Arc::new(png) });
        let bytes = save_package(&p).unwrap();
        let q = load_package(&bytes).unwrap();
        assert_eq!(q.graphs, p.graphs);
        assert_eq!(q.resources[0].bytes, p.resources[0].bytes);
        let img = decode(&q.resources[0].bytes).unwrap();
        assert!((img.get(0, 0, false)[1] - 0.5).abs() < 0.01);
        assert!(load_package(b"not a zip").is_err());
    }

    #[test]
    fn export_every_format() {
        let img = Image::generate(8, 8, Kind::Color, |x, y| [x as f32 / 7.0, y as f32 / 7.0, 0.5, 1.0]);
        let gray = img.to_gray();
        for (ext, _) in EXPORT_FORMATS {
            for depth in [Depth::U8, Depth::U16, Depth::F32] {
                for i in [&img, &gray] {
                    let b = encode(i, ext, depth, true).unwrap_or_else(|e| panic!("{ext}: {e}"));
                    let back = decode(&b).unwrap_or_else(|e| panic!("{ext} decode: {e}"));
                    assert_eq!((back.w, back.h), (8, 8));
                }
            }
        }
        let png16 = encode(&gray, "png", Depth::U16, false).unwrap();
        assert!((decode(&png16).unwrap().get(7, 0, false)[0] - gray.get(7, 0, false)[0]).abs() < 1e-3);
    }

    /// A valid 1-bit grayscale PNG of `side` x `side` zero pixels (a few hundred KB when deflated).
    fn blank_png(side: u32) -> Vec<u8> {
        fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
            out.extend((data.len() as u32).to_be_bytes());
            out.extend(kind);
            out.extend(data);
            let mut crc = flate2::Crc::new();
            crc.update(kind);
            crc.update(data);
            out.extend(crc.sum().to_be_bytes());
        }
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        let row = vec![0u8; 1 + (side as usize).div_ceil(8)];
        for _ in 0..side {
            z.write_all(&row).unwrap();
        }
        let mut ihdr = Vec::new();
        ihdr.extend(side.to_be_bytes());
        ihdr.extend(side.to_be_bytes());
        ihdr.extend([1, 0, 0, 0, 0]);
        let mut png = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'].to_vec();
        chunk(&mut png, b"IHDR", &ihdr);
        chunk(&mut png, b"IDAT", &z.finish().unwrap());
        chunk(&mut png, b"IEND", &[]);
        png
    }

    #[test]
    fn oversized_images_are_rejected_from_the_header_before_decoding() {
        // 30000 x 30000 is 900 MB as 8-bit gray: the size message (not an allocation failure
        // from the decoder) proves the header was checked before any pixels were decoded.
        let e = decode(&blank_png(30_000)).unwrap_err().to_string();
        assert!(e.contains("pixels"), "{e}");
        assert!(decode(&blank_png(64)).is_ok());
    }

    #[test]
    fn write_atomic_leaves_no_temp_file_and_overwrites() {
        let dir = std::env::temp_dir().join(format!("mc-atomic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.mcraft");
        write_atomic(&path, b"one").unwrap();
        write_atomic(&path, b"two").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"two");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn names_are_safe() {
        assert_eq!(output_name("$(graph)_$(usage)", "Bricks", "out", "baseColor"), "Bricks_baseColor");
        assert_eq!(safe_file_name("../../etc/passwd"), "passwd");
        assert_eq!(safe_file_name(".."), "resource");
    }
}
