use materialcraft_image::Image;
use std::hash::{DefaultHasher, Hash, Hasher};

pub(crate) fn image_key(i: &Image) -> u64 {
    let step = (i.data.len() / 4096).max(1);
    hash_of(&(i.w, i.h, i.data.iter().step_by(step).map(|v| v.to_bits()).collect::<Vec<u32>>()))
}

pub(crate) fn hash_of(v: &impl Hash) -> u64 {
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}
