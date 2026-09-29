use crate::failure::Failure;
use crate::pass::Range;
use crate::scan::Scan;
use crate::setting::{Setting, WIDTH, saturate};
use crate::shape::Shape;
use crate::store::Store;
use crate::table::Table;
use crate::work::Work;
use metal::device::{Command, Device, Kernel};
use photonic::laser::ground::{Exploration, Ground};
use photonic::runtime::Limit;

const SOURCE: &str = include_str!("kernel.metal");

pub fn group(kernel: &Kernel, thread: usize) -> [usize; 3] {
    [kernel.capacity().min(thread).max(1), 1, 1]
}

// What the passes of an exploration add up to: its events, whether a limit refused anything, and,
// when cycles matter, every marking's edges.
#[derive(Default)]
pub struct Tally {
    pub event: u64,
    pub refused: bool,
    pub outgoing: Vec<Vec<u32>>,
}

pub struct Engine {
    pub(crate) shape: Shape,
    pub(crate) device: Device,
    pub(crate) count: Kernel,
    pub(crate) expand: Kernel,
    pub(crate) insert: Kernel,
    pub(crate) resolve: Kernel,
    pub(crate) place: Kernel,
    pub(crate) rehash: Kernel,
    pub(crate) scan: Scan,
}

fn cyclic(outgoing: &[Vec<u32>]) -> bool {
    if outgoing.is_empty() {
        return false;
    }
    let mut color = vec![0u8; outgoing.len()];
    let mut stack = vec![(0usize, 0usize)];
    color[0] = 1;
    while let Some(&mut (node, ref mut position)) = stack.last_mut() {
        let Some(&next) = outgoing[node].get(*position) else {
            color[node] = 2;
            stack.pop();
            continue;
        };
        *position += 1;
        match color[next as usize] {
            0 => {
                color[next as usize] = 1;
                stack.push((next as usize, 0));
            }
            1 => return true,
            _ => {}
        }
    }
    false
}

impl Engine {
    pub fn new() -> Result<Self, Failure> {
        Self::shaped(Shape::default())
    }

    pub fn shaped(shape: Shape) -> Result<Self, Failure> {
        let device = Device::open()?;
        let library = device.library(SOURCE)?;
        let kernel = |entry: &str| device.kernel(&library, entry);
        Ok(Self {
            shape,
            count: kernel("count")?,
            expand: kernel("expand")?,
            insert: kernel("insert")?,
            resolve: kernel("resolve")?,
            place: kernel("place")?,
            rehash: kernel("rehash")?,
            scan: Scan::new(kernel("reduce")?, kernel("spread")?),
            device,
        })
    }

    pub fn name(&self) -> &str {
        self.device.name()
    }

    // Moves to a larger table when a pass could fill the table or would likely leave it more than
    // half full, since probes stay short below that, and tells whether the pass must enter every
    // marking into it first.
    pub(crate) fn grow(
        &self,
        store: &mut Store,
        safe: usize,
        likely: usize,
    ) -> Result<bool, Failure> {
        let wanted = (2 * likely).max(safe);
        if wanted <= store.slot {
            return Ok(false);
        }
        let slot = wanted.next_power_of_two();
        if slot > 1 << 32 {
            return Err(Failure::Configuration { count: likely });
        }
        store.table = self.device.space::<u32>(2 * slot)?;
        store.slot = slot;
        Ok(true)
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

    // Explores every schedule of plain events from the net's start in the order one thread searching
    // breadth first would, a window of markings at a time, until no new marking appears; the limits
    // refuse what they refuse, and past the configuration limit only known markings are reached.
    // With cycle, it also keeps every edge to tell whether a run can go on forever.
    pub fn explore(
        &self,
        ground: &mut Ground,
        limit: Limit,
        cycle: bool,
    ) -> Result<Exploration, Failure> {
        let mut table = Table::new(ground);
        table.prepare(ground, &ground.start())?;
        let mut upload = None;
        let mut store = Store::new(&self.device, &ground.start(), self.shape)?;
        let mut work = Work::new(&self.device, self.shape.initial)?;
        let mut tally = Tally::default();
        let mut end = Vec::new();
        let mut cursor = 0;
        let mut counted = false;
        while cursor < store.count {
            let size = self.shape.window.min(store.count - cursor);
            let window = self.count(
                ground,
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
                    let to = window.bound(start, from, self.shape.pass as u64);
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
        if cycle {
            tally.outgoing.resize(store.count, Vec::new());
        }
        Ok(Exploration {
            closed: !tally.refused,
            configuration: store.count,
            event: tally.event,
            end: end.into_iter().map(|id| store.marking(id)).collect(),
            endless: cycle.then(|| cyclic(&tally.outgoing)),
        })
    }
}
