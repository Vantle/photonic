// FNV-1a over the bytes alone, so a streamed report and a buffered one give the same value on
// every platform and toolchain, which std's unspecified DefaultHasher does not promise.
const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fingerprint(u64);

impl Default for Fingerprint {
    fn default() -> Self {
        Self(OFFSET)
    }
}

impl Fingerprint {
    pub fn update(&mut self, byte: &[u8]) {
        for &value in byte {
            self.0 = (self.0 ^ u64::from(value)).wrapping_mul(PRIME);
        }
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
#[path = "test/fingerprint.rs"]
mod test;
