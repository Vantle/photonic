use super::map;
use super::passage::Passage;
use crate::basis::Set;
use crate::executor::Executor;
use crate::place::Place;
use hashing::Builder;
use indexmap::{Equivalent, IndexSet};
use std::hash::{Hash, Hasher};

// Traces carried along many paths share their bases, so bases and coherence sets are named by a
// number. A single place or world is its own number, and its image across an event is computed
// where it is needed; only larger sets are interned, shard by shard, and each event remembers its
// image of every one carried across it or of a single one whose image is larger. A carry reads
// only its own event's small table; each round first fills the tables with the images it lacks,
// found by their contents without building a set.
const SHARD: usize = 64;

const SINGLE: u64 = 1 << 63;

trait Single: Copy + Ord + Hash {
    fn encode(self) -> Option<u64>;
    fn decode(value: u64) -> Self;
}

impl Single for usize {
    fn encode(self) -> Option<u64> {
        let value = u64::try_from(self).ok()?;
        (value < SINGLE).then_some(SINGLE | value)
    }

    fn decode(value: u64) -> Self {
        (value & !SINGLE) as Self
    }
}

impl Single for Place {
    fn encode(self) -> Option<u64> {
        let (kind, container, id) = match self {
            Self::World(container, id) => (0, container, id),
            Self::Context(container, id) => (1, container, id),
            Self::Held(container, id) => (2, container, id),
        };
        let container = u64::try_from(container)
            .ok()
            .filter(|&value| value < 1 << 29)?;
        let id = u64::try_from(id).ok().filter(|&value| value < 1 << 32)?;
        Some(SINGLE | kind << 61 | container << 32 | id)
    }

    fn decode(value: u64) -> Self {
        let container = ((value >> 32) & ((1 << 29) - 1)) as usize;
        let id = (value & ((1 << 32) - 1)) as usize;
        match (value >> 61) & 3 {
            0 => Self::World(container, id),
            1 => Self::Context(container, id),
            _ => Self::Held(container, id),
        }
    }
}

pub(super) enum View<'pool, Value> {
    Single(Value),
    Many(&'pool [Value]),
}

impl<Value: Copy> View<'_, Value> {
    pub fn len(&self) -> usize {
        match self {
            Self::Single(_) => 1,
            Self::Many(value) => value.len(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Value> + '_ {
        let (single, many) = match self {
            Self::Single(value) => (Some(*value), &[][..]),
            Self::Many(value) => (None, *value),
        };
        single.into_iter().chain(many.iter().copied())
    }
}

struct Entry<Value> {
    hash: u64,
    set: Set<Value>,
}

struct Probe<'slice, Value> {
    hash: u64,
    slice: &'slice [Value],
}

impl<Value> Hash for Entry<Value> {
    fn hash<Target: Hasher>(&self, hasher: &mut Target) {
        hasher.write_u64(self.hash);
    }
}

impl<Value: Eq> PartialEq for Entry<Value> {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.set == other.set
    }
}

impl<Value: Eq> Eq for Entry<Value> {}

impl<Value> Hash for Probe<'_, Value> {
    fn hash<Target: Hasher>(&self, hasher: &mut Target) {
        hasher.write_u64(self.hash);
    }
}

impl<Value: Eq> Equivalent<Entry<Value>> for Probe<'_, Value> {
    fn equivalent(&self, key: &Entry<Value>) -> bool {
        self.hash == key.hash && key.set.iter().as_slice() == self.slice
    }
}

#[derive(Default)]
struct Image {
    basis: Vec<(u64, u64)>,
    world: Vec<(u64, u64)>,
}

#[derive(Default)]
pub(super) struct Demand {
    basis: Vec<u64>,
    world: Vec<u64>,
}

enum Found<Value> {
    Known(u64),
    New(u64, Set<Value>),
}

struct Computed {
    event: usize,
    basis: Vec<(u64, Found<Place>)>,
    world: Vec<(u64, Found<usize>)>,
}

pub(super) struct Pool {
    basis: Vec<IndexSet<Entry<Place>, Builder>>,
    world: Vec<IndexSet<Entry<usize>, Builder>>,
    image: Vec<Image>,
}

impl Default for Pool {
    fn default() -> Self {
        Self {
            basis: (0..SHARD).map(|_| IndexSet::default()).collect(),
            world: (0..SHARD).map(|_| IndexSet::default()).collect(),
            image: Vec::new(),
        }
    }
}

fn digest<Value: Hash>(slice: &[Value]) -> u64 {
    hashing::value(&slice)
}

// Tables inside a shard place entries by the low bits of the same hash, so shards take middle
// bits that the tables do not use.
fn slot(hash: u64) -> usize {
    (hash >> 40) as usize % SHARD
}

fn identify(shard: usize, local: usize) -> u64 {
    let id = u64::try_from(local * SHARD + shard).expect("fewer than 2^63 sets");
    assert!(id < SINGLE, "fewer than 2^63 sets");
    id
}

fn view<Value: Single>(shard: &[IndexSet<Entry<Value>, Builder>], id: u64) -> View<'_, Value> {
    if id & SINGLE != 0 {
        return View::Single(Value::decode(id));
    }
    let id = id as usize;
    View::Many(shard[id % SHARD][id / SHARD].set.iter().as_slice())
}

fn find<Value: Single>(shard: &[IndexSet<Entry<Value>, Builder>], slice: &[Value]) -> Found<Value> {
    if let [single] = slice
        && let Some(code) = single.encode()
    {
        return Found::Known(code);
    }
    let hash = digest(slice);
    let position = slot(hash);
    match shard[position].get_index_of(&Probe { hash, slice }) {
        Some(local) => Found::Known(identify(position, local)),
        None => Found::New(hash, slice.iter().copied().collect()),
    }
}

fn image<Value: Single>(
    shard: &[IndexSet<Entry<Value>, Builder>],
    set: &View<'_, Value>,
    buffer: &mut Vec<Value>,
    part: impl Fn(Value) -> Set<Value>,
) -> Found<Value> {
    buffer.clear();
    for value in set.iter() {
        buffer.extend(part(value).iter().copied());
    }
    if set.len() > 1 {
        buffer.sort_unstable();
        buffer.dedup();
    }
    find(shard, buffer)
}

fn direct<Value: Single>(id: u64, part: impl Fn(Value) -> Set<Value>) -> Option<u64> {
    if id & SINGLE == 0 {
        return None;
    }
    match part(Value::decode(id)).iter().as_slice() {
        [single] => single.encode(),
        _ => None,
    }
}

fn intern<Value: Eq + Send + Sync>(
    executor: Option<&Executor>,
    shard: &mut [IndexSet<Entry<Value>, Builder>],
    fresh: Vec<(u64, Set<Value>)>,
) -> Vec<u64> {
    let count = fresh.len();
    if count == 0 {
        return Vec::new();
    }
    let mut group = (0..SHARD).map(|_| Vec::new()).collect::<Vec<_>>();
    for (position, (hash, set)) in fresh.into_iter().enumerate() {
        group[slot(hash)].push((position, Entry { hash, set }));
    }
    let taken = group
        .into_iter()
        .enumerate()
        .filter(|(_, list)| !list.is_empty())
        .map(|(index, list)| (index, std::mem::take(&mut shard[index]), list))
        .collect::<Vec<_>>();
    let interned = map(executor, taken, |(index, mut set, list)| {
        let found = list
            .into_iter()
            .map(|(position, entry)| (position, identify(index, set.insert_full(entry).0)))
            .collect::<Vec<_>>();
        (index, set, found)
    });
    let mut id = vec![0; count];
    for (index, set, list) in interned {
        shard[index] = set;
        for (position, found) in list {
            id[position] = found;
        }
    }
    id
}

fn resolve<Value: Eq + Send + Sync>(
    executor: Option<&Executor>,
    shard: &mut [IndexSet<Entry<Value>, Builder>],
    mut found: Vec<Vec<(u64, Found<Value>)>>,
) -> Vec<Vec<(u64, u64)>> {
    let mut fresh = Vec::new();
    for (_, value) in found.iter_mut().flatten() {
        if let Found::New(hash, set) = value {
            fresh.push((*hash, std::mem::take(set)));
        }
    }
    let mut interned = intern(executor, shard, fresh).into_iter();
    found
        .into_iter()
        .map(|list| {
            list.into_iter()
                .map(|(source, value)| match value {
                    Found::Known(id) => (source, id),
                    Found::New(..) => (source, interned.next().expect("one id per new set")),
                })
                .collect()
        })
        .collect()
}

fn lookup(table: &[(u64, u64)], id: u64) -> Option<u64> {
    let position = table.binary_search_by_key(&id, |&(key, _)| key).ok()?;
    Some(table[position].1)
}

fn merge(table: &mut Vec<(u64, u64)>, found: Vec<(u64, u64)>) {
    table.extend(found);
    table.sort_unstable_by_key(|&(key, _)| key);
}

fn add<Value: Eq + Hash + Single>(
    shard: &mut [IndexSet<Entry<Value>, Builder>],
    set: Set<Value>,
) -> u64 {
    if let [single] = set.iter().as_slice()
        && let Some(code) = single.encode()
    {
        return code;
    }
    let hash = digest(set.iter().as_slice());
    let position = slot(hash);
    identify(position, shard[position].insert_full(Entry { hash, set }).0)
}

impl Demand {
    fn settle(&mut self) {
        self.basis.sort_unstable();
        self.basis.dedup();
        self.world.sort_unstable();
        self.world.dedup();
    }
}

impl Pool {
    pub fn basis(&mut self, basis: Set<Place>) -> u64 {
        add(&mut self.basis, basis)
    }

    pub fn world(&mut self, world: Set<usize>) -> u64 {
        add(&mut self.world, world)
    }

    pub fn release(&mut self) {
        self.image = Vec::new();
    }

    pub fn place(&self, basis: u64) -> View<'_, Place> {
        view(&self.basis, basis)
    }

    pub fn site(&self, world: u64) -> View<'_, usize> {
        view(&self.world, world)
    }

    pub fn carry(&self, basis: u64, event: usize, flow: &Passage) -> u64 {
        direct(basis, |place| flow.resource(place)).unwrap_or_else(|| {
            lookup(&self.image[event].basis, basis)
                .expect("every image is prepared before a carry reads it")
        })
    }

    pub fn follow(&self, world: u64, event: usize, flow: &Passage) -> u64 {
        direct(world, |index| flow.context(index)).unwrap_or_else(|| {
            lookup(&self.image[event].world, world)
                .expect("every image is prepared before a carry reads it")
        })
    }

    pub fn demand(
        &self,
        event: usize,
        basis: impl Iterator<Item = u64>,
        world: u64,
        flow: &Passage,
        demand: &mut Demand,
    ) {
        let image = self.image.get(event);
        for id in basis {
            if direct(id, |place| flow.resource(place)).is_some()
                || image.is_some_and(|image| lookup(&image.basis, id).is_some())
            {
                continue;
            }
            demand.basis.push(id);
        }
        if direct(world, |index| flow.context(index)).is_none()
            && image.is_none_or(|image| lookup(&image.world, world).is_none())
        {
            demand.world.push(world);
        }
    }

    pub fn prepare<'flow>(
        &mut self,
        executor: Option<&Executor>,
        demand: Vec<(usize, Demand)>,
        flow: impl Fn(usize) -> &'flow Passage + Sync + Send,
    ) {
        let computed = map(executor, demand, |(event, mut demand)| {
            demand.settle();
            let flow = flow(event);
            let mut place = Vec::new();
            let mut site = Vec::new();
            Computed {
                event,
                basis: demand
                    .basis
                    .into_iter()
                    .map(|id| {
                        let part = |value: Place| flow.resource(value);
                        (id, image(&self.basis, &self.place(id), &mut place, part))
                    })
                    .collect(),
                world: demand
                    .world
                    .into_iter()
                    .map(|id| {
                        let part = |value: usize| flow.context(value);
                        (id, image(&self.world, &self.site(id), &mut site, part))
                    })
                    .collect(),
            }
        });
        let mut event = Vec::with_capacity(computed.len());
        let mut basis = Vec::with_capacity(computed.len());
        let mut world = Vec::with_capacity(computed.len());
        for value in computed {
            event.push(value.event);
            basis.push(value.basis);
            world.push(value.world);
        }
        let basis = resolve(executor, &mut self.basis, basis);
        let world = resolve(executor, &mut self.world, world);
        for ((event, basis), world) in event.into_iter().zip(basis).zip(world) {
            if self.image.len() <= event {
                self.image.resize_with(event + 1, Image::default);
            }
            let image = &mut self.image[event];
            merge(&mut image.basis, basis);
            merge(&mut image.world, world);
        }
    }
}
