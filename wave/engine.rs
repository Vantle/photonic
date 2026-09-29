use crate::failure::Failure;
use crate::scan::Scan;
use crate::search::Search;
use crate::setting::{BACKWARD, REFUSED, Setting, WIDTH, prelude, saturate};
use crate::store::Store;
use crate::table::Table;
use crate::tally::Tally;
use crate::tuning::Tuning;
use crate::upload::Upload;
use crate::work::Work;
use metal::device::{Command, Device, Kernel};
use photonic::laser::net::{Cycle, Exploration, Net};
use photonic::runtime::Limit;

// The kernels' sources, each beside the host code it serves, in the order they declare what later
// ones use.
const SOURCE: [&str; 9] = [
    include_str!("setting.metal"),
    include_str!("arena.metal"),
    include_str!("table.metal"),
    include_str!("hash.metal"),
    include_str!("stream.metal"),
    include_str!("scan.metal"),
    include_str!("window.metal"),
    include_str!("pass.metal"),
    include_str!("store.metal"),
];

// Explores nets on the GPU through Metal, with its kernels compiled once for every exploration.
pub struct Engine {
    pub(crate) tuning: Tuning,
    pub(crate) device: Device,
    pub(crate) count: Kernel,
    pub(crate) expand: Kernel,
    pub(crate) insert: Kernel,
    pub(crate) tally: Kernel,
    pub(crate) bound: Kernel,
    pub(crate) place: Kernel,
    pub(crate) point: Kernel,
    pub(crate) weigh: Kernel,
    pub(crate) rehash: Kernel,
    pub(crate) scan: Scan,
}

impl Engine {
    // The engine on the system's GPU, or none when there is no GPU whose kernels can reach memory
    // through device addresses.
    pub fn new() -> Result<Option<Self>, Failure> {
        Self::tuned(Tuning::default())
    }

    pub(crate) fn tuned(tuning: Tuning) -> Result<Option<Self>, Failure> {
        let device = match Device::open() {
            Ok(device) => device,
            Err(metal::failure::Failure::Unavailable(_)) => return Ok(None),
            Err(failure) => return Err(failure.into()),
        };
        if !device.addressing() {
            return Ok(None);
        }
        let library = device.library(&(prelude() + &SOURCE.concat()))?;
        let kernel = |entry: &str| device.kernel(&library, entry);
        Ok(Some(Self {
            tuning,
            count: kernel("count")?,
            expand: kernel("expand")?,
            insert: kernel("insert")?,
            tally: kernel("tally")?,
            bound: kernel("bound")?,
            place: kernel("place")?,
            point: kernel("point")?,
            weigh: kernel("weigh")?,
            rehash: kernel("rehash")?,
            scan: Scan::new(kernel("reduce")?, kernel("spread")?),
            device,
        }))
    }

    // The GPU the engine explores on.
    pub fn name(&self) -> &str {
        self.device.name()
    }

    // Explores every schedule of plain events from the net's start in the order one thread searching
    // breadth first would, a window of markings at a time, until no new marking appears; the limits
    // refuse what they refuse, and past the configuration limit only known markings are reached.
    // It agrees with the net's own exploration number for number, and searches the edges for a
    // cycle only when one leads to a marking found no later than its source.
    pub fn explore(
        &self,
        net: &mut Net,
        limit: Limit,
        cycle: Cycle,
    ) -> Result<Exploration, Failure> {
        let start = net.start();
        let mut table = Table::new(net);
        table.prepare(net, &start)?;
        let mut search = Search {
            upload: Upload::new(&self.device, &table)?,
            store: Store::new(&self.device, &start, self.tuning)?,
            work: Work::new(&self.device, self.tuning.initial)?,
            tally: Tally::default(),
            table,
            net,
            limit,
            cycle,
        };
        let mut end = Vec::new();
        let mut cursor = 0;
        let mut counted = false;
        while cursor < search.store.count {
            let size = self.tuning.window.min(search.store.count - cursor);
            let window = self.count(&mut search, cursor, size, counted)?;
            counted = false;
            end.extend(&window.end);
            let mut from = 0;
            while from < size {
                let range = window.range(
                    search.work.start.memory.view::<u64>(),
                    from,
                    self.tuning.pass as u64,
                );
                from = range.to;
                counted = self.pass(
                    &mut search,
                    &window,
                    range,
                    (from == size).then_some(cursor + size),
                )?;
            }
            cursor += size;
        }
        let summary = search.work.summary.view::<u32>();
        let (refused, backward) = (summary[REFUSED] != 0, summary[BACKWARD] != 0);
        Ok(Exploration {
            closed: !refused,
            configuration: search.store.count,
            event: search.tally.event,
            end: end.into_iter().map(|id| search.store.marking(id)).collect(),
            endless: (cycle == Cycle::Find).then(|| backward && search.tally.cyclic()),
        })
    }

    // Encodes entering every established marking into a table just grown.
    pub(crate) fn fill<'device>(
        &self,
        command: &mut Command<'device>,
        search: &'device Search<'_>,
    ) -> Result<(), Failure> {
        let store = &search.store;
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
        );
        Ok(())
    }
}
