#![forbid(unsafe_code)]

use std::hash::{BuildHasher, Hash};

const FACTOR: u64 = 0xf135_7aea_2e62_a9c5;
const GOLDEN: u64 = 0x9e37_79b9_7f4a_7c15;

#[derive(Clone, Copy, Debug, Default)]
pub struct Builder;

#[derive(Clone, Copy, Debug, Default)]
pub struct Hasher(u64);

impl BuildHasher for Builder {
    type Hasher = Hasher;

    #[inline]
    fn build_hasher(&self) -> Hasher {
        Hasher::default()
    }
}

impl std::hash::Hasher for Hasher {
    #[inline]
    fn write(&mut self, byte: &[u8]) {
        let (chunk, rest) = byte.as_chunks::<8>();
        for &word in chunk {
            self.write_u64(u64::from_le_bytes(word));
        }
        if rest.is_empty() {
            return;
        }
        let mut word = [0; 8];
        word[..rest.len()].copy_from_slice(rest);
        self.write_u64(u64::from_le_bytes(word));
    }

    #[inline]
    fn write_u8(&mut self, value: u8) {
        self.write_u64(value.into());
    }

    #[inline]
    fn write_u16(&mut self, value: u16) {
        self.write_u64(value.into());
    }

    #[inline]
    fn write_u32(&mut self, value: u32) {
        self.write_u64(value.into());
    }

    #[inline]
    fn write_u64(&mut self, value: u64) {
        self.0 = self.0.wrapping_add(value).wrapping_mul(FACTOR);
    }

    #[inline]
    fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0.rotate_left(26)
    }
}

#[inline]
pub fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[inline]
pub fn combine(seed: u64, value: u64) -> u64 {
    mix(seed.rotate_left(17) ^ value.wrapping_add(GOLDEN))
}

#[inline]
pub fn value(value: &impl Hash) -> u64 {
    Builder.hash_one(value)
}

#[cfg(test)]
#[path = "test.rs"]
mod test;
