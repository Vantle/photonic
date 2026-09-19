#[derive(Clone, Copy, Default)]
pub(crate) struct Accumulator {
    sum: u64,
    square: u64,
    count: u64,
}

impl Accumulator {
    pub fn insert(&mut self, value: u64) {
        self.sum = self.sum.wrapping_add(value);
        self.square = self.square.wrapping_add(value.wrapping_mul(value));
        self.count += 1;
    }

    pub fn remove(&mut self, value: u64) {
        self.sum = self.sum.wrapping_sub(value);
        self.square = self.square.wrapping_sub(value.wrapping_mul(value));
        self.count -= 1;
    }

    pub fn value(&self) -> u64 {
        crate::fingerprint::mix(self.sum)
            .wrapping_add(crate::fingerprint::mix(self.square).rotate_left(21))
            .wrapping_add(crate::fingerprint::mix(self.count).rotate_left(42))
    }
}
