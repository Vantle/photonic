use crate::failure::Failure;
use crate::grow::Grow;
use metal::device::{Device, Memory};

// The memory one exploration reuses from window to window and pass to pass: the counts and running
// sums of a window's markings, the markings it flags, the words of successors the host found, each
// candidate's record, each source's sum of kind terms, each candidate's target, slot and share, the
// events each threadgroup stands for, the winners and words of each threadgroup of candidates, the
// scan levels, a summary of flags and the scans' totals.
pub struct Work {
    pub number: Grow<u64>,
    pub start: Grow<u64>,
    pub flagged: Grow<u32>,
    pub extra: Grow<u32>,
    pub record: Grow<u32>,
    pub sum: Grow<u64>,
    pub target: Grow<u32>,
    pub slot: Grow<u32>,
    pub share: Grow<u32>,
    pub event: Grow<u64>,
    pub block: Grow<u64>,
    pub level: Vec<Grow<u64>>,
    pub summary: Memory,
    pub total: Memory,
}

impl Work {
    pub fn new(device: &Device, initial: usize) -> Result<Self, Failure> {
        Ok(Self {
            number: Grow::new(device, initial)?,
            start: Grow::new(device, initial)?,
            flagged: Grow::new(device, 2 * initial)?,
            extra: Grow::new(device, 0)?,
            record: Grow::new(device, 3 * initial)?,
            sum: Grow::new(device, initial)?,
            target: Grow::new(device, initial)?,
            slot: Grow::new(device, initial)?,
            share: Grow::new(device, initial)?,
            event: Grow::new(device, 0)?,
            block: Grow::new(device, 0)?,
            level: Vec::new(),
            summary: device.memory::<u32>(4)?,
            total: device.memory::<u64>(3)?,
        })
    }
}
