use crate::arena::{self, Arena};
use crate::engine::Engine;
use crate::failure::Failure;
use crate::grow::Grow;
use crate::hash;
use crate::setting::{NONE, Setting, WIDTH, saturate};
use crate::tuning::Tuning;
use metal::device::{Command, Device, Memory};
use photonic::laser::net::Marking;

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
    pub fn new(device: &Device, start: &Marking, tuning: Tuning) -> Result<Self, Failure> {
        let word = arena::word(start).collect::<Vec<_>>();
        let arena = Arena::new(device, &word, tuning.first, tuning.largest)?;
        let mut offset = Grow::<u64>::new(device, tuning.initial)?;
        offset.memory.edit::<u64>()[0] = 0;
        let slot = tuning.initial.next_power_of_two().max(WIDTH);
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

    // Moves to a larger table when a pass could fill the table or would likely leave it more than
    // half full, since probes stay short below that, and tells whether the pass must enter every
    // marking into it first. Every slot's position stays below NONE, which marks a candidate that
    // claimed no slot.
    pub fn grow(&mut self, device: &Device, safe: usize, likely: usize) -> Result<bool, Failure> {
        let wanted = (2 * likely).max(safe);
        if wanted <= self.slot {
            return Ok(false);
        }
        let slot = wanted.next_power_of_two();
        if slot > NONE as usize {
            return Err(Failure::Configuration { count: likely });
        }
        self.table = device.space::<u32>(2 * slot)?;
        self.slot = slot;
        Ok(true)
    }
}

impl Engine {
    // Encodes entering every established marking into a table just grown.
    pub(crate) fn fill<'device>(
        &self,
        command: &mut Command<'device>,
        store: &'device Store,
    ) -> Result<(), Failure> {
        let setting = Setting {
            count: saturate(store.count),
            bucket: saturate(store.slot / WIDTH),
            ..Setting::default()
        };
        command.clear(&store.table)?;
        command.dispatch(
            &self.rehash,
            &[&store.arena.address, &store.offset.memory, &store.table],
            &setting.byte(),
            [store.count, 1, 1],
            self.rehash.group([store.count, 1, 1]),
        )?;
        Ok(())
    }
}
