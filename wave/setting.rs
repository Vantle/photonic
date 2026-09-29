// Words the kernels and the host share.
pub const BLOCKED: u32 = u32::MAX - 1;
pub const TAG: u32 = 1 << 31;
pub const LIMITED: u32 = 1 << 31;
pub const JOINED: u32 = 1 << 30;
pub const COPIES: u32 = JOINED - 1;
pub const SHIFT: u32 = 40;
pub const WORDS: u64 = (1 << SHIFT) - 1;
pub const CANDIDATE: usize = 1 << (64 - SHIFT);
pub const MISSING: u32 = 1;
pub const JOIN: u32 = 2;
pub const BARE: u32 = 4;
pub const FLAGGED: usize = 0;
// The scans' totals: a pass's winners and words, then a window's candidates.
pub const SUM: u32 = 1;
pub const REFUSED: usize = 1;
pub const WIDTH: usize = 8;

// The values every kernel reads, laid out as the kernels' Setting.
#[derive(Clone, Copy, Default)]
pub struct Setting {
    pub base: u64,
    pub arena: u64,
    pub split: u64,
    pub overflow: u64,
    pub room: u64,
    pub first: u32,
    pub count: u32,
    pub shift: u32,
    pub key: u32,
    pub root: u32,
    pub rule: u32,
    pub wide: u32,
    pub coherence: u32,
    pub occurrence: u32,
    pub scope: u32,
    pub bucket: u32,
    pub next: u32,
    pub allowed: u32,
}

impl Setting {
    // The kernels' layout rounds the structure up to its eight-byte alignment.
    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(96);
        for value in [self.base, self.arena, self.split, self.overflow, self.room] {
            bytes.extend(value.to_le_bytes());
        }
        for value in [
            self.first,
            self.count,
            self.shift,
            self.key,
            self.root,
            self.rule,
            self.wide,
            self.coherence,
            self.occurrence,
            self.scope,
            self.bucket,
            self.next,
            self.allowed,
        ] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.resize(bytes.len().next_multiple_of(8), 0);
        bytes
    }
}

// A running sum's values: how many, whether each tile adds its partial, and which total slot the
// whole sum goes to.
#[derive(Clone, Copy, Default)]
pub struct Span {
    pub count: u32,
    pub add: u32,
    pub slot: u32,
}

impl Span {
    pub fn bytes(&self) -> Vec<u8> {
        [self.count, self.add, self.slot]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }
}

pub fn saturate(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
