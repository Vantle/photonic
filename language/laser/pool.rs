use super::map;
use super::passage::Passage;
use crate::basis::Set;
use crate::executor::Executor;
use crate::place::Place;
use hashing::Builder;
use indexmap::{Equivalent, IndexSet};
use std::hash::{Hash, Hasher};

// Traces carried along many paths share their bases, so bases and coherence sets are interned
// once and each event remembers its image of every interned set carried across it. A carry reads
// only its own event's small table; each round first fills the tables with the images it lacks.
// Images are found by their contents without building a set, and only new sets are interned,
// shard by shard.
const SHARD: usize = 64;

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
    basis: Vec<(u32, u32)>,
    world: Vec<(u32, u32)>,
}

#[derive(Default)]
pub(super) struct Demand {
    basis: Vec<u32>,
    world: Vec<u32>,
}

enum Found<Value> {
    Known(u32),
    New(u64, Set<Value>),
}

struct Computed {
    event: usize,
    basis: Vec<(u32, Found<Place>)>,
    world: Vec<(u32, Found<usize>)>,
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

fn identify(shard: usize, local: usize) -> u32 {
    u32::try_from(local * SHARD + shard).expect("fewer than 2^32 sets")
}

fn find<Value: Copy + Ord + Hash>(
    shard: &[IndexSet<Entry<Value>, Builder>],
    slice: &[Value],
) -> Found<Value> {
    let hash = digest(slice);
    let position = slot(hash);
    match shard[position].get_index_of(&Probe { hash, slice }) {
        Some(local) => Found::Known(identify(position, local)),
        None => Found::New(hash, slice.iter().copied().collect()),
    }
}

fn image<Value: Copy + Ord + Hash>(
    shard: &[IndexSet<Entry<Value>, Builder>],
    set: &Set<Value>,
    buffer: &mut Vec<Value>,
    part: impl Fn(&Value) -> Set<Value>,
) -> Found<Value> {
    buffer.clear();
    for value in set {
        buffer.extend(part(value).iter().copied());
    }
    if set.len() > 1 {
        buffer.sort_unstable();
        buffer.dedup();
    }
    find(shard, buffer)
}

fn intern<Value: Eq + Send + Sync>(
    executor: Option<&Executor>,
    shard: &mut [IndexSet<Entry<Value>, Builder>],
    fresh: Vec<(u64, Set<Value>)>,
) -> Vec<u32> {
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
    mut found: Vec<Vec<(u32, Found<Value>)>>,
) -> Vec<Vec<(u32, u32)>> {
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

fn lookup(table: &[(u32, u32)], id: u32) -> Option<u32> {
    let position = table.binary_search_by_key(&id, |&(key, _)| key).ok()?;
    Some(table[position].1)
}

fn merge(table: &mut Vec<(u32, u32)>, found: Vec<(u32, u32)>) {
    table.extend(found);
    table.sort_unstable_by_key(|&(key, _)| key);
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
    pub fn basis(&mut self, basis: Set<Place>) -> u32 {
        let hash = digest(basis.iter().as_slice());
        let position = slot(hash);
        let entry = Entry { hash, set: basis };
        identify(position, self.basis[position].insert_full(entry).0)
    }

    pub fn world(&mut self, world: Set<usize>) -> u32 {
        let hash = digest(world.iter().as_slice());
        let position = slot(hash);
        let entry = Entry { hash, set: world };
        identify(position, self.world[position].insert_full(entry).0)
    }

    pub fn release(&mut self) {
        self.image = Vec::new();
    }

    pub fn place(&self, basis: u32) -> &Set<Place> {
        let basis = basis as usize;
        &self.basis[basis % SHARD][basis / SHARD].set
    }

    pub fn site(&self, world: u32) -> &Set<usize> {
        let world = world as usize;
        &self.world[world % SHARD][world / SHARD].set
    }

    pub fn carry(&self, basis: u32, event: usize) -> u32 {
        lookup(&self.image[event].basis, basis)
            .expect("every image is prepared before a carry reads it")
    }

    pub fn follow(&self, world: u32, event: usize) -> u32 {
        lookup(&self.image[event].world, world)
            .expect("every image is prepared before a carry reads it")
    }

    pub fn demand(
        &self,
        event: usize,
        basis: impl Iterator<Item = u32>,
        world: u32,
        demand: &mut Demand,
    ) {
        let Some(image) = self.image.get(event) else {
            demand.basis.extend(basis);
            demand.world.push(world);
            return;
        };
        demand
            .basis
            .extend(basis.filter(|&id| lookup(&image.basis, id).is_none()));
        if lookup(&image.world, world).is_none() {
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
                        let part = |value: &Place| flow.resource(*value);
                        (id, image(&self.basis, self.place(id), &mut place, part))
                    })
                    .collect(),
                world: demand
                    .world
                    .into_iter()
                    .map(|id| {
                        let part = |value: &usize| flow.context(*value);
                        (id, image(&self.world, self.site(id), &mut site, part))
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
