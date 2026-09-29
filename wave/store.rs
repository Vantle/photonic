use crate::arena::Arena;
use crate::failure::Failure;
use crate::grow::Grow;
use crate::hash;
use crate::setting::WIDTH;
use crate::shape::Shape;
use metal::device::{Device, Memory};
use photonic::laser::ground::Marking;

// Every marking found so far: its words in the arena, where each starts, and the table that finds a
// marking by its contents, two words a slot, with the share of the last pass's candidates that
// were new markings, which sizes the table for the next.
pub struct Store {
    pub arena: Arena,
    pub offset: Grow<u64>,
    pub table: Memory,
    pub slot: usize,
    pub count: usize,
    pub fresh: f64,
}

impl Store {
    pub fn new(device: &Device, start: &Marking, shape: Shape) -> Result<Self, Failure> {
        let mut words = vec![start.root, start.kind.len() as u32];
        words.extend(&start.kind);
        let arena = Arena::new(device, &words, shape.first, shape.largest)?;
        let mut offset = Grow::<u64>::new(device, shape.initial)?;
        offset.memory.edit::<u64>()[0] = 0;
        let slot = shape.initial.next_power_of_two().max(WIDTH);
        let mut table = device.memory::<u32>(2 * slot)?;
        let digest = hash::digest(start.root, &start.kind);
        let position = (digest as usize & (slot / WIDTH - 1)) * WIDTH;
        table.edit::<u32>()[2 * position..2 * position + 2]
            .copy_from_slice(&[1, (digest >> 32) as u32 | 1]);
        Ok(Self {
            arena,
            offset,
            table,
            slot,
            count: 1,
            fresh: 1.0,
        })
    }

    pub fn marking(&mut self, id: usize) -> Marking {
        let offset = self.offset.memory.view::<u64>()[id];
        self.arena.marking(offset)
    }
}
