use crate::dispatch::group;
use crate::failure::Failure;
use crate::pass::Range;
use crate::scan::Scan;
use crate::setting::{Setting, WIDTH, saturate};
use crate::store::Store;
use crate::table::Table;
use crate::tally::Tally;
use crate::tuning::Tuning;
use crate::work::Work;
use metal::device::{Command, Device, Kernel};
use photonic::laser::net::{Cycle, Exploration, Net};
use photonic::runtime::Limit;

const SOURCE: &str = include_str!("kernel.metal");

// Explores nets on the GPU through Metal, with its kernels compiled once for every exploration.
pub struct Engine {
    pub(crate) tuning: Tuning,
    pub(crate) device: Device,
    pub(crate) count: Kernel,
    pub(crate) expand: Kernel,
    pub(crate) insert: Kernel,
    pub(crate) tally: Kernel,
    pub(crate) place: Kernel,
    pub(crate) point: Kernel,
    pub(crate) rehash: Kernel,
    pub(crate) scan: Scan,
}

impl Engine {
    pub fn new() -> Result<Self, Failure> {
        Self::tuned(Tuning::default())
    }

    pub(crate) fn tuned(tuning: Tuning) -> Result<Self, Failure> {
        let device = Device::open()?;
        let library = device.library(SOURCE)?;
        let kernel = |entry: &str| device.kernel(&library, entry);
        Ok(Self {
            tuning,
            count: kernel("count")?,
            expand: kernel("expand")?,
            insert: kernel("insert")?,
            tally: kernel("tally")?,
            place: kernel("place")?,
            point: kernel("point")?,
            rehash: kernel("rehash")?,
            scan: Scan::new(kernel("reduce")?, kernel("spread")?),
            device,
        })
    }

    // The GPU the engine explores on.
    pub fn name(&self) -> &str {
        self.device.name()
    }

    // Explores every schedule of plain events from the net's start in the order one thread searching
    // breadth first would, a window of markings at a time, until no new marking appears; the limits
    // refuse what they refuse, and past the configuration limit only known markings are reached.
    // It agrees with the net's own exploration number for number.
    pub fn explore(
        &self,
        net: &mut Net,
        limit: Limit,
        cycle: Cycle,
    ) -> Result<Exploration, Failure> {
        let mut table = Table::new(net);
        table.prepare(net, &net.start())?;
        let mut upload = None;
        let mut store = Store::new(&self.device, &net.start(), self.tuning)?;
        let mut work = Work::new(&self.device, self.tuning.initial)?;
        let mut tally = Tally::default();
        let mut end = Vec::new();
        let mut cursor = 0;
        let mut counted = false;
        while cursor < store.count {
            let size = self.tuning.window.min(store.count - cursor);
            let window = self.count(
                net,
                &mut table,
                &mut upload,
                &mut store,
                &mut work,
                cursor,
                size,
                limit,
                counted,
            )?;
            counted = false;
            end.extend(&window.end);
            let mut from = 0;
            while from < size {
                let range = {
                    let start = work.start.memory.view::<u64>();
                    let to = window.bound(start, from, self.tuning.pass as u64);
                    Range {
                        from,
                        to,
                        begin: window.before(start, from),
                        end: window.before(start, to),
                    }
                };
                from = range.to;
                counted = self.pass(
                    &mut table,
                    &mut upload,
                    &mut store,
                    &mut work,
                    &window,
                    range,
                    (from == size).then_some(cursor + size),
                    limit,
                    cycle,
                    &mut tally,
                )?;
            }
            cursor += size;
        }
        Ok(Exploration {
            closed: !tally.refused,
            configuration: store.count,
            event: tally.event,
            end: end.into_iter().map(|id| store.marking(id)).collect(),
            endless: (cycle == Cycle::Find).then(|| tally.cyclic(store.count)),
        })
    }

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
            &setting.bytes(),
            [store.count, 1, 1],
            group(&self.rehash, store.count),
        );
        Ok(())
    }
}
