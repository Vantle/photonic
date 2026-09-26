use crate::link::Link;
use std::hash::{Hash, Hasher};

pub(crate) fn label(value: &impl Hash) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hash);
    hash.finish()
}

#[inline]
pub(crate) fn edge(kind: Link, value: u64) -> u64 {
    hashing::mix(value.wrapping_add(hashing::mix(kind as u64)))
}
