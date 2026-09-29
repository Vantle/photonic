use crate::failure::Failure;
use metal::device::{Device, Memory};
use photonic::laser::net::Marking;

// The most segments an arena holds, as the kernels' Arena lays them out.
pub const SEGMENT: usize = 1024;

// Markings' words in segments that never move, so the arena grows without copying and each segment
// is committed once; the kernels reach the segments through their device addresses. An offset names
// a segment in its high half and a word within it in its low half.
pub struct Arena {
    pub segment: Vec<Memory>,
    pub address: Memory,
    capacity: usize,
    used: usize,
    largest: usize,
}

// Where a pass's new markings go: those whose spots come before the split follow the last segment's
// words from the base, and the rest start the next segment at the overflow.
pub struct Place {
    pub base: u64,
    pub split: u64,
    pub overflow: u64,
}

fn encode(segment: usize, word: usize) -> u64 {
    (segment as u64) << 32 | word as u64
}

impl Arena {
    // The first segment takes first words, and each later one twice as many as the one before, up
    // to largest words unless one pass needs more.
    pub fn new(
        device: &Device,
        words: &[u32],
        first: usize,
        largest: usize,
    ) -> Result<Self, Failure> {
        let capacity = first.max(words.len());
        let mut segment = device.space::<u32>(capacity)?;
        segment.edit::<u32>()[..words.len()].copy_from_slice(words);
        let mut address = device.memory::<u64>(SEGMENT)?;
        address.edit::<u64>()[0] = segment.address();
        Ok(Self {
            segment: vec![segment],
            address,
            capacity,
            used: words.len(),
            largest,
        })
    }

    // Where the next marking would start in the last segment, and how many words the segment has
    // left.
    pub fn base(&self) -> u64 {
        encode(self.segment.len() - 1, self.used)
    }

    pub fn room(&self) -> usize {
        self.capacity - self.used
    }

    pub fn advance(&mut self, words: usize) {
        self.used += words;
    }

    // Places a pass's new markings that take more words than the last segment has left: those in
    // the first kept words stay in the last segment, and the rest start a new one.
    pub fn split(&mut self, device: &Device, kept: u64, words: usize) -> Result<Place, Failure> {
        if self.segment.len() == SEGMENT {
            return Err(Failure::Arena { words });
        }
        let split = kept;
        let rest = words - split as usize;
        let capacity = (2 * self.capacity).min(self.largest).max(rest);
        let fresh = device.space::<u32>(capacity)?;
        self.address.edit::<u64>()[self.segment.len()] = fresh.address();
        let place = Place {
            base: self.base(),
            split,
            overflow: encode(self.segment.len(), 0),
        };
        self.segment.push(fresh);
        self.capacity = capacity;
        self.used = rest;
        Ok(place)
    }

    pub fn marking(&mut self, offset: u64) -> Marking {
        let words = self.segment[(offset >> 32) as usize].view::<u32>();
        let start = (offset & 0xffff_ffff) as usize;
        let length = words[start + 1] as usize;
        Marking {
            root: words[start],
            kind: words[start + 2..start + 2 + length].to_vec(),
        }
    }
}
