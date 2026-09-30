use photonic::laser::net;
use photonic::stop::Bound;

// Words the kernels and the host share; the kernels read them as constants of the same names.
pub const EMPTY: u32 = u32::MAX;
pub const NONE: u32 = u32::MAX;
pub const BLOCKED: u32 = u32::MAX - 1;
pub const TAG: u32 = 1 << 31;
pub const LIMITED: u32 = 1 << 31;
pub const JOINED: u32 = 1 << 30;
pub const COPY: u32 = JOINED - 1;
pub const SHIFT: u32 = 40;
pub const WORD: u64 = (1 << SHIFT) - 1;
pub const CANDIDATE: usize = 1 << (64 - SHIFT);
pub const MISSING: u32 = 1;
pub const JOIN: u32 = 2;
pub const BARE: u32 = 4;
pub const HEADER: usize = 9;
// The words that describe a part of several components: its root, where its kinds start among the
// members and how many, and where its entries start and how many.
pub const PART: usize = 5;
pub const WIDTH: usize = 8;
pub const SEGMENT: usize = 1024;
// The most offers of a marking's kinds to the inputs of joining rules, and of distinct parts they
// join, the most picks its joining rules make, the most inputs of a rule and the most choices of
// kinds and copies a marking's joining events are found from on the GPU; a marking that needs more
// is joined on the host. Past the net's own offers and picks, the host's join takes work.
pub const OFFER: usize = net::OFFER;
pub const PICK: usize = net::PICK;
pub const ARITY: usize = 8;
pub const STEP: usize = 4096;
// The most SIMD groups in a threadgroup whose kernel sums them in one SIMD group; its scratch keeps
// a sum for each and the group's whole sum after them.
pub const BAND: usize = 32;
// The summary's words: how many markings a count flagged, whether a candidate found a marking no
// later than its source, and from REFUSAL on the events each limit refused, as two words, low then
// high, for the configuration, coherence, occurrence and scope limits in turn, the order the host
// numbers them in.
pub const FLAGGED: usize = 0;
pub const BACKWARD: usize = 1;
pub const REFUSAL: usize = 2;
pub const SUMMARY: usize = REFUSAL + 2 * photonic::stop::BOUND.len();
pub const CONFIGURATION: u32 = Bound::Configuration as u32;
pub const COHERENCE: u32 = Bound::Coherence as u32;
pub const OCCURRENCE: u32 = Bound::Occurrence as u32;
pub const SCOPE: u32 = Bound::Scope as u32;
// The totals' slots: a pass's winners and their words, a window's candidates, and, when the
// configuration limit can cut a pass short, the words of its admitted winners at most.
pub const WINNER: usize = 0;
pub const WINDOW: usize = 1;
pub const BOUND: usize = 2;

// Declares constants to the kernels by the names and values the host gives them.
macro_rules! declare {
    ($kind:literal, $($name:ident),+) => {
        [$(format!("constant {} {} = {};\n", $kind, stringify!($name), $name)),+].concat()
    };
}

// The constants every kernel can read, declared ahead of the kernels' source.
pub fn prelude() -> String {
    declare!(
        "uint",
        EMPTY,
        NONE,
        BLOCKED,
        TAG,
        LIMITED,
        JOINED,
        COPY,
        SHIFT,
        CANDIDATE,
        MISSING,
        JOIN,
        BARE,
        HEADER,
        PART,
        WIDTH,
        SEGMENT,
        OFFER,
        PICK,
        ARITY,
        STEP,
        BAND,
        FLAGGED,
        BACKWARD,
        REFUSAL,
        CONFIGURATION,
        COHERENCE,
        OCCURRENCE,
        SCOPE,
        WINNER,
        WINDOW,
        BOUND
    ) + &declare!("ulong", WORD)
}

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
    pub catalog: u32,
    pub rule: u32,
    pub join: u32,
    pub coherence: u32,
    pub occurrence: u32,
    pub scope: u32,
    pub bucket: u32,
    pub next: u32,
    pub allowed: u32,
}

impl Setting {
    // The kernels' layout rounds the structure up to its eight-byte alignment.
    pub fn byte(&self) -> Vec<u8> {
        let mut byte = Vec::with_capacity(96);
        for value in [self.base, self.arena, self.split, self.overflow, self.room] {
            byte.extend(value.to_le_bytes());
        }
        for value in [
            self.first,
            self.count,
            self.shift,
            self.key,
            self.root,
            self.catalog,
            self.rule,
            self.join,
            self.coherence,
            self.occurrence,
            self.scope,
            self.bucket,
            self.next,
            self.allowed,
        ] {
            byte.extend(value.to_le_bytes());
        }
        byte.resize(byte.len().next_multiple_of(8), 0);
        byte
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
    pub fn byte(&self) -> Vec<u8> {
        [self.count, self.add, self.slot]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }
}

pub fn saturate(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
