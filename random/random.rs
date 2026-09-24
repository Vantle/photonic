#![forbid(unsafe_code)]

#[derive(Clone, Debug)]
pub struct Generator {
    state: [u64; 4],
}

fn mix(value: &mut u64) -> u64 {
    *value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut result = *value;
    result = (result ^ (result >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    result = (result ^ (result >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    result ^ (result >> 31)
}

impl Generator {
    pub fn new(seed: u64) -> Self {
        let mut value = seed;
        Self {
            state: [
                mix(&mut value),
                mix(&mut value),
                mix(&mut value),
                mix(&mut value),
            ],
        }
    }

    pub fn integer(&mut self) -> u64 {
        let result = self.state[0]
            .wrapping_add(self.state[3])
            .rotate_left(23)
            .wrapping_add(self.state[0]);
        let shifted = self.state[1] << 17;
        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];
        self.state[2] ^= shifted;
        self.state[3] = self.state[3].rotate_left(45);
        result
    }

    pub fn split(&mut self) -> Self {
        Self::new(self.integer())
    }

    pub fn uniform(&mut self) -> f64 {
        (self.integer() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn below(&mut self, bound: usize) -> usize {
        assert!(bound > 0, "a draw needs a nonempty range");
        ((u128::from(self.integer()) * bound as u128) >> 64) as usize
    }

    pub fn chance(&mut self, probability: f64) -> bool {
        self.uniform() < probability
    }

    pub fn normal(&mut self) -> f64 {
        let radius = (-2.0 * (1.0 - self.uniform()).ln()).sqrt();
        radius * (std::f64::consts::TAU * self.uniform()).cos()
    }

    pub fn gumbel(&mut self) -> f64 {
        -(-(1.0 - self.uniform()).ln()).ln()
    }

    pub fn shuffle<T>(&mut self, value: &mut [T]) {
        for index in (1..value.len()).rev() {
            value.swap(index, self.below(index + 1));
        }
    }

    pub fn choose<'value, T>(&mut self, value: &'value [T]) -> Option<&'value T> {
        if value.is_empty() {
            return None;
        }
        value.get(self.below(value.len()))
    }

    pub fn weighted(&mut self, weight: &[f64]) -> Option<usize> {
        let total = weight.iter().filter(|value| **value > 0.0).sum::<f64>();
        if total <= 0.0 {
            return None;
        }
        let mut remaining = self.uniform() * total;
        for (index, value) in weight.iter().enumerate() {
            if *value <= 0.0 {
                continue;
            }
            if remaining < *value {
                return Some(index);
            }
            remaining -= value;
        }
        weight.iter().rposition(|value| *value > 0.0)
    }
}

#[cfg(test)]
#[path = "test.rs"]
mod test;
