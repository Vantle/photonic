use std::hash::{Hash, Hasher};

pub(crate) fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

pub(crate) fn value(value: &impl Hash) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hash);
    hash.finish()
}

pub(crate) fn edge(kind: u8, value: u64) -> u64 {
    mix(value.wrapping_add(mix(kind as u64)))
}
