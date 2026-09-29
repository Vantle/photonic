use super::layout::{Layout, Site};
use super::transition::Effect;
use crate::basis::Set;
use crate::flow::Flow;
use crate::place::Place;
use smallvec::SmallVec;
use std::num::NonZeroU32;
use std::sync::Arc;

// A passage answers, for each place, world and frame after an event, where it came from before.
// A flat passage keeps an event's own flow as renamings plus the places that do not follow them.
// A composed passage keeps the parts the event did not touch as moves between layouts and asks
// the event's effect about the parts it made and the root.
pub(super) enum Passage {
    Flat(Box<Flat>),
    Composed(Composed),
}

pub(super) struct Flat {
    frame: Vec<Option<usize>>,
    world: Vec<Option<NonZeroU32>>,
    token: Vec<Option<NonZeroU32>>,
    resource: Vec<(Place, Set<Place>)>,
    context: Vec<(usize, Set<usize>)>,
}

fn encode(value: usize) -> Option<NonZeroU32> {
    NonZeroU32::new(u32::try_from(value + 1).ok()?)
}

fn decode(value: Option<NonZeroU32>) -> Option<usize> {
    value.map(|value| value.get() as usize - 1)
}

fn token(place: Place) -> usize {
    match place {
        Place::World(_, id) | Place::Context(_, id) | Place::Held(_, id) => id,
    }
}

fn rebuild(place: Place, container: usize, id: usize) -> Place {
    match place {
        Place::World(..) => Place::World(container, id),
        Place::Context(..) => Place::Context(container, id),
        Place::Held(..) => Place::Held(container, id),
    }
}

// The one value of a set, when it holds exactly one.
fn single<Value: Copy>(set: &Set<Value>) -> Option<Value> {
    match set.iter().as_slice() {
        [single] => Some(*single),
        _ => None,
    }
}

// The value a list sorted by key keeps for a key.
fn lookup<'list, Key: Ord, Value>(list: &'list [(Key, Value)], key: &Key) -> Option<&'list Value> {
    let position = list
        .binary_search_by(|(candidate, _)| candidate.cmp(key))
        .ok()?;
    Some(&list[position].1)
}

// Where each part after an event comes from: a part before it, or one the event's transition made.
// Every event keeps one per part, so they are small.
#[derive(Clone, Copy)]
pub(super) enum Origin {
    Same(u32),
    Produced(u32),
}

pub(super) struct Composed {
    pub source: Arc<Layout>,
    pub target: Arc<Layout>,
    pub origin: Box<[Origin]>,
    pub involved: SmallVec<[usize; 4]>,
    pub effect: Arc<Effect>,
}

// Where the parts an event touched lie in its whole source: the effect's layout of them alone, the
// source's layout, and the source's part each of them is.
struct Embedding<'composed> {
    sub: &'composed Layout,
    source: &'composed Layout,
    involved: &'composed [usize],
}

impl Embedding<'_> {
    fn place(&self, place: Place) -> Place {
        match self.sub.site(place) {
            Site::Root => place,
            Site::Part(position) => self
                .sub
                .shift(position, self.source, self.involved[position])
                .place(place),
        }
    }

    fn world(&self, world: usize) -> usize {
        let position = self.sub.world(world);
        self.sub
            .shift(position, self.source, self.involved[position])
            .world(world)
    }

    fn frame(&self, frame: usize) -> usize {
        match self.sub.frame(frame) {
            Site::Root => frame,
            Site::Part(position) => self
                .sub
                .shift(position, self.source, self.involved[position])
                .frame(frame),
        }
    }
}

impl Passage {
    pub fn resource(&self, place: Place) -> Set<Place> {
        match self {
            Self::Flat(flat) => flat.resource(place),
            Self::Composed(composed) => composed.resource(place),
        }
    }

    pub fn context(&self, world: usize) -> Set<usize> {
        match self {
            Self::Flat(flat) => flat.context(world),
            Self::Composed(composed) => composed.context(world),
        }
    }

    pub fn frame(&self, frame: usize) -> Option<usize> {
        match self {
            Self::Flat(flat) => flat.frame(frame),
            Self::Composed(composed) => composed.frame(frame),
        }
    }

    // The one place a place came from, or none when it came from several or from nothing; most
    // places come from one, so carrying them needs no set.
    pub fn place(&self, place: Place) -> Option<Place> {
        match self {
            Self::Flat(flat) => flat.place(place),
            Self::Composed(composed) => composed.place(place),
        }
    }

    pub fn world(&self, world: usize) -> Option<usize> {
        match self {
            Self::Flat(flat) => flat.world(world),
            Self::Composed(composed) => composed.world(world),
        }
    }
}

impl Composed {
    fn local(&self) -> (Embedding<'_>, &Layout, &Flat) {
        let embedding = Embedding {
            sub: &self.effect.source,
            source: &self.source,
            involved: &self.involved,
        };
        (embedding, &self.effect.result, &self.effect.passage)
    }

    fn place(&self, place: Place) -> Option<Place> {
        let (embedding, result, passage) = self.local();
        let Site::Part(part) = self.target.site(place) else {
            return passage.place(place).map(|value| embedding.place(value));
        };
        match self.origin[part] {
            Origin::Same(other) => Some(
                self.target
                    .shift(part, &self.source, other as usize)
                    .place(place),
            ),
            Origin::Produced(index) => {
                let local = self.target.shift(part, result, index as usize).place(place);
                passage.place(local).map(|value| embedding.place(value))
            }
        }
    }

    fn world(&self, world: usize) -> Option<usize> {
        let (embedding, result, passage) = self.local();
        let part = self.target.world(world);
        match self.origin[part] {
            Origin::Same(other) => Some(
                self.target
                    .shift(part, &self.source, other as usize)
                    .world(world),
            ),
            Origin::Produced(index) => {
                let local = self.target.shift(part, result, index as usize).world(world);
                passage.world(local).map(|value| embedding.world(value))
            }
        }
    }

    fn resource(&self, place: Place) -> Set<Place> {
        let (embedding, result, passage) = self.local();
        let Site::Part(part) = self.target.site(place) else {
            return passage
                .resource(place)
                .iter()
                .map(|&value| embedding.place(value))
                .collect();
        };
        match self.origin[part] {
            Origin::Same(other) => Set::single(
                self.target
                    .shift(part, &self.source, other as usize)
                    .place(place),
            ),
            Origin::Produced(index) => {
                let local = self.target.shift(part, result, index as usize).place(place);
                passage
                    .resource(local)
                    .iter()
                    .map(|&value| embedding.place(value))
                    .collect()
            }
        }
    }

    fn context(&self, world: usize) -> Set<usize> {
        let (embedding, result, passage) = self.local();
        let part = self.target.world(world);
        match self.origin[part] {
            Origin::Same(other) => Set::single(
                self.target
                    .shift(part, &self.source, other as usize)
                    .world(world),
            ),
            Origin::Produced(index) => {
                let local = self.target.shift(part, result, index as usize).world(world);
                passage
                    .context(local)
                    .iter()
                    .map(|&value| embedding.world(value))
                    .collect()
            }
        }
    }

    fn frame(&self, frame: usize) -> Option<usize> {
        let (embedding, result, passage) = self.local();
        let Site::Part(part) = self.target.frame(frame) else {
            return passage.frame(frame).map(|value| embedding.frame(value));
        };
        match self.origin[part] {
            Origin::Same(other) => Some(
                self.target
                    .shift(part, &self.source, other as usize)
                    .frame(frame),
            ),
            Origin::Produced(index) => passage
                .frame(self.target.shift(part, result, index as usize).frame(frame))
                .map(|value| embedding.frame(value)),
        }
    }
}

impl Flat {
    pub fn new(flow: Flow) -> Self {
        let world = flow
            .context
            .iter()
            .map(|set| single(set).and_then(encode))
            .collect::<Vec<_>>();
        let context = flow
            .context
            .into_iter()
            .enumerate()
            .filter(|(_, set)| set.len() != 1)
            .collect();
        let count = flow
            .resource
            .iter()
            .map(|(place, _)| token(*place) + 1)
            .max()
            .unwrap_or(0);
        let mut passage = Self {
            frame: flow.frame,
            world,
            token: vec![None; count],
            resource: Vec::new(),
            context,
        };
        for (place, set) in flow.resource {
            if single(&set).is_some_and(|source| passage.propose(place, source)) {
                continue;
            }
            passage.resource.push((place, set));
        }
        passage
    }

    fn container(&self, place: Place) -> Option<usize> {
        match place {
            Place::World(index, _) => decode(*self.world.get(index)?),
            Place::Context(index, _) | Place::Held(index, _) => *self.frame.get(index)?,
        }
    }

    fn propose(&mut self, place: Place, source: Place) -> bool {
        let ((Place::World(_, id), Place::World(container, origin))
        | (Place::Context(_, id), Place::Context(container, origin))
        | (Place::Held(_, id), Place::Held(container, origin))) = (place, source)
        else {
            return false;
        };
        if self.container(place) != Some(container) {
            return false;
        }
        match decode(self.token[id]) {
            Some(known) => known == origin,
            None => {
                self.token[id] = encode(origin);
                self.token[id].is_some()
            }
        }
    }

    // The place a kept place came from, through the renamings.
    fn follow(&self, place: Place) -> Place {
        let container = self
            .container(place)
            .expect("a place the flow keeps has a container");
        let id = decode(self.token[token(place)]).expect("a place the flow keeps has a token");
        rebuild(place, container, id)
    }

    fn resource(&self, place: Place) -> Set<Place> {
        match lookup(&self.resource, &place) {
            Some(set) => set.clone(),
            None => Set::single(self.follow(place)),
        }
    }

    fn context(&self, world: usize) -> Set<usize> {
        match lookup(&self.context, &world) {
            Some(set) => set.clone(),
            None => {
                Set::single(decode(self.world[world]).expect("a world the flow keeps has a source"))
            }
        }
    }

    fn frame(&self, frame: usize) -> Option<usize> {
        self.frame[frame]
    }

    fn place(&self, place: Place) -> Option<Place> {
        match lookup(&self.resource, &place) {
            Some(set) => single(set),
            None => Some(self.follow(place)),
        }
    }

    fn world(&self, world: usize) -> Option<usize> {
        match lookup(&self.context, &world) {
            Some(set) => single(set),
            None => Some(decode(self.world[world]).expect("a world the flow keeps has a source")),
        }
    }
}
