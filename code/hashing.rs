const FACTOR: u64 = 0xf135_7aea_2e62_a9c5;

#[derive(Clone, Copy, Debug, Default)]
pub struct Builder;

#[derive(Clone, Copy, Debug, Default)]
pub struct Hasher(u64);

impl std::hash::BuildHasher for Builder {
    type Hasher = Hasher;

    fn build_hasher(&self) -> Hasher {
        Hasher::default()
    }
}

impl std::hash::Hasher for Hasher {
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

    fn write_u8(&mut self, value: u8) {
        self.write_u64(value.into());
    }

    fn write_u16(&mut self, value: u16) {
        self.write_u64(value.into());
    }

    fn write_u32(&mut self, value: u32) {
        self.write_u64(value.into());
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = self.0.wrapping_add(value).wrapping_mul(FACTOR);
    }

    fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }

    fn finish(&self) -> u64 {
        self.0.rotate_left(26)
    }
}

pub fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

pub fn combine(seed: u64, value: u64) -> u64 {
    mix(seed.rotate_left(17) ^ value.wrapping_add(0x9e37_79b9_7f4a_7c15))
}

pub fn value(value: &impl std::hash::Hash) -> u64 {
    use std::hash::BuildHasher;
    Builder.hash_one(value)
}
