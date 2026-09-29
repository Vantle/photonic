use crate::failure::Failure;
use crate::setting::{SEGMENT, saturate};
use metal::device::{Device, Memory};
use photonic::laser::net::Marking;

// Markings' words in segments that never move, so the arena grows without copying and each segment
// is committed once; the kernels reach the segments through their device addresses. An offset names
// a segment in its high half and a word within it in its low half.
pub struct Arena {
    pub segment: Vec<Memory>,
    pub address: Memory,
    used: usize,
    largest: usize,
}

fn encode(segment: usize, word: usize) -> u64 {
    (segment as u64) << 32 | word as u64
}

// The words a marking takes in the arena: its root, its number of kinds and its sorted kinds.
pub fn word(marking: &Marking) -> impl Iterator<Item = u32> {
    [marking.root, saturate(marking.kind.len())]
        .into_iter()
        .chain(marking.kind.iter().copied())
}

impl Arena {
    // The first segment takes first words, and each later one twice as many as the one before, up
    // to largest words unless one pass needs more.
    pub fn new(
        device: &Device,
        word: &[u32],
        first: usize,
        largest: usize,
    ) -> Result<Self, Failure> {
        let mut segment = device.space::<u32>(first.max(word.len()))?;
        segment.edit::<u32>()[..word.len()].copy_from_slice(word);
        let mut address = device.memory::<u64>(SEGMENT)?;
        address.edit::<u64>()[0] = segment.address();
        Ok(Self {
            segment: vec![segment],
            address,
            used: word.len(),
            largest,
        })
    }

    fn capacity(&self) -> usize {
        self.segment[self.segment.len() - 1].length() / std::mem::size_of::<u32>()
    }

    // Where the next marking would start in the last segment, and how many words the segment has
    // left.
    pub fn base(&self) -> u64 {
        encode(self.segment.len() - 1, self.used)
    }

    pub fn room(&self) -> usize {
        self.capacity() - self.used
    }

    pub fn advance(&mut self, word: usize) {
        self.used += word;
    }

    // Places a pass's new markings that take more words than the last segment has left: those in
    // the first kept words stay in the last segment, and the rest start a new one, whose first word
    // it gives.
    pub fn split(&mut self, device: &Device, kept: u64, word: usize) -> Result<u64, Failure> {
        if self.segment.len() == SEGMENT {
            return Err(Failure::Arena { word });
        }
        let rest = word - kept as usize;
        let fresh = device.space::<u32>((2 * self.capacity()).min(self.largest).max(rest))?;
        self.address.edit::<u64>()[self.segment.len()] = fresh.address();
        self.segment.push(fresh);
        self.used = rest;
        Ok(encode(self.segment.len() - 1, 0))
    }

    pub fn marking(&mut self, offset: u64) -> Marking {
        let word = self.segment[(offset >> 32) as usize].view::<u32>();
        let start = (offset & 0xffff_ffff) as usize;
        let length = word[start + 1] as usize;
        Marking {
            root: word[start],
            kind: word[start + 2..start + 2 + length].to_vec(),
        }
    }
}
