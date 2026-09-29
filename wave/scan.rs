use crate::dispatch::whole;
use crate::failure::Failure;
use crate::grow::Grow;
use crate::setting::{Span, saturate};
use metal::device::{Command, Device, Kernel, Memory};

// Exclusive running sums of 64-bit values on the device. A tile is four values a thread; each level
// sums the tiles of the one below, until one tile remains, whose pass writes the whole sum.
pub struct Scan {
    reduce: Kernel,
    spread: Kernel,
    width: usize,
}

impl Scan {
    pub fn new(reduce: Kernel, spread: Kernel) -> Self {
        let width = whole(&reduce).min(whole(&spread));
        Self {
            reduce,
            spread,
            width,
        }
    }

    fn tile(&self) -> usize {
        4 * self.width
    }

    // Makes room in each level for the partial sums of a scan of count values.
    pub fn prepare(
        &self,
        device: &Device,
        level: &mut Vec<Grow<u64>>,
        count: usize,
    ) -> Result<(), Failure> {
        let mut size = count;
        let mut depth = 0;
        while size > self.tile() {
            size = size.div_ceil(self.tile());
            if level.len() == depth {
                level.push(Grow::new(device, size)?);
            }
            level[depth].fit(device, size)?;
            depth += 1;
        }
        Ok(())
    }

    // Replaces count values with their exclusive running sum and writes the whole sum to a slot of
    // total; the levels were prepared for at least count values.
    pub fn encode<'device>(
        &self,
        command: &mut Command<'device>,
        value: &'device Memory,
        count: usize,
        total: &'device Memory,
        slot: usize,
        level: &'device [Grow<u64>],
    ) {
        if count <= self.tile() {
            let span = Span {
                count: saturate(count),
                add: 0,
                slot: saturate(slot),
            };
            command.dispatch(
                &self.spread,
                &[value, value, total],
                &span.byte(),
                [self.width, 1, 1],
                [self.width, 1, 1],
            );
            return;
        }
        let group = count.div_ceil(self.tile());
        let partial = &level[0].memory;
        let span = Span {
            count: saturate(count),
            add: 1,
            slot: saturate(slot),
        };
        command.dispatch(
            &self.reduce,
            &[value, partial],
            &span.byte(),
            [group * self.width, 1, 1],
            [self.width, 1, 1],
        );
        self.encode(command, partial, group, total, slot, &level[1..]);
        command.dispatch(
            &self.spread,
            &[value, partial, total],
            &span.byte(),
            [group * self.width, 1, 1],
            [self.width, 1, 1],
        );
    }
}
