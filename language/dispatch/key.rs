use std::ops::RangeInclusive;

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Key {
    pub frame: usize,
    pub input: usize,
    pub owner: usize,
}

impl Key {
    pub fn frame(frame: usize) -> RangeInclusive<Self> {
        Self {
            frame,
            input: 0,
            owner: 0,
        }..=Self {
            frame,
            input: usize::MAX,
            owner: usize::MAX,
        }
    }

    pub fn input(frame: usize, input: usize) -> RangeInclusive<Self> {
        Self {
            frame,
            input,
            owner: 0,
        }..=Self {
            frame,
            input,
            owner: usize::MAX,
        }
    }
}
