use crate::arena::Arena;
use crate::dispatch::whole;
use crate::failure::Failure;
use crate::scan::Scan;
use crate::search::Search;
use crate::setting::{BACKWARD, REFUSED, prelude};
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
const SOURCE: [&str; 10] = [
    include_str!("setting.metal"),
    include_str!("arena.metal"),
    include_str!("table.metal"),
    include_str!("hash.metal"),
    include_str!("stream.metal"),
    include_str!("join.metal"),
    include_str!("scan.metal"),
    include_str!("window.metal"),
    include_str!("pass.metal"),
    include_str!("store.metal"),
];

// Explores nets on the GPU through Metal, with its kernels compiled once for every exploration,
// and the threads of a group that expand markings, one a thread, and that take candidates, four a
// thread.
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
    pub(crate) width: usize,
    pub(crate) span: usize,
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
        let expand = kernel("expand")?;
        let tally = kernel("tally")?;
        let place = kernel("place")?;
        let weigh = kernel("weigh")?;
        let width = whole(&expand);
        // Tallying and placing must share their groups, since placing ranks each winner within the
        // group tallying summed.
        let span = whole(&tally).min(whole(&place)).min(whole(&weigh));
        Ok(Some(Self {
            tuning,
            count: kernel("count")?,
            expand,
            insert: kernel("insert")?,
            tally,
            bound: kernel("bound")?,
            place,
            point: kernel("point")?,
            weigh,
            rehash: kernel("rehash")?,
            scan: Scan::new(kernel("reduce")?, kernel("spread")?),
            width,
            span,
            device,
        }))
    }

    // The GPU the engine explores on.
    pub fn name(&self) -> &str {
        self.device.name()
    }

    // A command whose kernels can reach every segment of the arena.
    pub(crate) fn command<'device>(
        &'device self,
        arena: &'device Arena,
    ) -> Result<Command<'device>, Failure> {
        let mut command = self.device.command()?;
        command.reach(&arena.segment);
        Ok(command)
    }

    // Explores every schedule of plain events from the net's start in the order one thread searching
    // breadth first would, a window of markings at a time, until no new marking appears; the limits
    // refuse what they refuse, past the configuration limit only known markings are reached, and
    // the budget stops it before the first marking whose parts it cannot ground. It agrees with the
    // net's own exploration number for number, and searches the edges for a cycle only when one
    // leads to a marking found no later than its source.
    pub fn explore(
        &self,
        net: &mut Net,
        budget: usize,
        limit: Limit,
        cycle: Cycle,
    ) -> Result<Exploration, Failure> {
        let work = net.work();
        let allowance = work.saturating_add(budget);
        let start = net.start();
        let mut table = Table::new(net);
        if table.prepare(net, &start, allowance)?.is_none() {
            return Ok(Exploration {
                closed: false,
                configuration: 1,
                event: 0,
                end: Vec::new(),
                endless: (cycle == Cycle::Find).then_some(false),
                work: net.work() - work,
            });
        }
        let mut search = Search {
            upload: Upload::new(&self.device, &table)?,
            store: Store::new(&self.device, &start, self.tuning)?,
            work: Work::new(&self.device, self.tuning.initial)?,
            tally: Tally::default(),
            table,
            net,
            allowance,
            limit,
            cycle,
        };
        let mut end = Vec::new();
        let mut cursor = 0;
        let mut counted = false;
        let mut spent = false;
        while cursor < search.store.count && !spent {
            let size = self.tuning.window.min(search.store.count - cursor);
            let window = self.window(&mut search, cursor, size, counted)?;
            spent = window.spent;
            end.extend(&window.end);
            let mut from = 0;
            while from < window.size {
                let range = window.range(
                    search.work.start.memory.view::<u64>(),
                    from,
                    self.tuning.pass as u64,
                );
                from = range.to;
                let ahead = (from == window.size && !spent).then_some(cursor + window.size);
                counted = self.pass(&mut search, &window, range, ahead)?;
            }
            cursor += window.size;
        }
        let summary = search.work.summary.view::<u32>();
        let (refused, backward) = (summary[REFUSED] != 0, summary[BACKWARD] != 0);
        let count = search.store.count;
        Ok(Exploration {
            closed: !refused && !spent,
            configuration: count,
            event: search.tally.event,
            end: end.into_iter().map(|id| search.store.marking(id)).collect(),
            endless: (cycle == Cycle::Find).then(|| backward && search.tally.cyclic(count)),
            work: search.net.work() - work,
        })
    }
}
