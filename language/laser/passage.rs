use super::layout::{Layout, Site};
use super::transition::Transition;
use crate::basis::Set;
use crate::flow::Flow;
use crate::place::Place;
use smallvec::SmallVec;
use std::num::NonZeroU32;
use std::sync::Arc;

// A passage answers, for each place, world and frame after an event, where it came from before.
// A flat passage keeps an event's own flow as renamings plus the places that do not follow them.
// A composed passage keeps the parts the event did not touch as moves between layouts and asks
// the event's transition about the parts it made and the root.
pub(super) enum Passage {
    Flat(Flat),
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

fn part(place: Place) -> (usize, usize) {
    match place {
        Place::World(index, id) | Place::Context(index, id) | Place::Held(index, id) => (index, id),
    }
}

fn rebuild(place: Place, container: usize, id: usize) -> Place {
    match place {
        Place::World(..) => Place::World(container, id),
        Place::Context(..) => Place::Context(container, id),
        Place::Held(..) => Place::Held(container, id),
    }
}

#[derive(Clone, Copy)]
pub(super) enum Origin {
    Same(usize),
    Produced(usize),
}

pub(super) struct Composed {
    pub source: Arc<Layout>,
    pub target: Arc<Layout>,
    pub origin: Vec<Origin>,
    pub involved: SmallVec<[usize; 4]>,
    pub transition: Arc<Transition>,
}

impl Passage {
    pub fn new(flow: Flow) -> Self {
        Self::Flat(Flat::new(flow))
    }

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
            Self::Flat(flat) => flat.frame[frame],
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
    fn local(&self) -> (&Layout, &Layout, &Passage) {
        let Transition::Local(effect) = &*self.transition else {
            unreachable!("a composed passage holds a local transition")
        };
        (&effect.source, &effect.result, &effect.passage)
    }

    fn lift(&self, sub: &Layout, place: Place) -> Place {
        match sub.site(place) {
            Site::Root => place,
            Site::Part(position) => {
                sub.move_place(place, position, &self.source, self.involved[position])
            }
        }
    }

    fn back(&self, sub: &Layout, set: &Set<Place>) -> Set<Place> {
        set.iter().map(|&place| self.lift(sub, place)).collect()
    }

    fn place(&self, place: Place) -> Option<Place> {
        let (sub, result, passage) = self.local();
        let Site::Part(part) = self.target.site(place) else {
            return passage.place(place).map(|value| self.lift(sub, value));
        };
        match self.origin[part] {
            Origin::Same(other) => Some(self.target.move_place(place, part, &self.source, other)),
            Origin::Produced(index) => {
                let local = self.target.move_place(place, part, result, index);
                passage.place(local).map(|value| self.lift(sub, value))
            }
        }
    }

    fn world(&self, world: usize) -> Option<usize> {
        let (sub, result, passage) = self.local();
        let part = self.target.world(world);
        match self.origin[part] {
            Origin::Same(other) => Some(self.target.move_world(world, part, &self.source, other)),
            Origin::Produced(index) => {
                let local = self.target.move_world(world, part, result, index);
                passage.world(local).map(|value| {
                    let position = sub.world(value);
                    sub.move_world(value, position, &self.source, self.involved[position])
                })
            }
        }
    }

    fn resource(&self, place: Place) -> Set<Place> {
        let (sub, result, passage) = self.local();
        let Site::Part(part) = self.target.site(place) else {
            return self.back(sub, &passage.resource(place));
        };
        match self.origin[part] {
            Origin::Same(other) => {
                Set::single(self.target.move_place(place, part, &self.source, other))
            }
            Origin::Produced(index) => {
                let local = self.target.move_place(place, part, result, index);
                self.back(sub, &passage.resource(local))
            }
        }
    }

    fn context(&self, world: usize) -> Set<usize> {
        let (sub, result, passage) = self.local();
        let part = self.target.world(world);
        match self.origin[part] {
            Origin::Same(other) => {
                Set::single(self.target.move_world(world, part, &self.source, other))
            }
            Origin::Produced(index) => {
                let local = self.target.move_world(world, part, result, index);
                passage
                    .context(local)
                    .iter()
                    .map(|&value| {
                        let position = sub.world(value);
                        sub.move_world(value, position, &self.source, self.involved[position])
                    })
                    .collect()
            }
        }
    }

    fn frame(&self, frame: usize) -> Option<usize> {
        let (sub, result, passage) = self.local();
        let back = |value: usize| match sub.frame(value) {
            Site::Root => value,
            Site::Part(position) => {
                sub.move_frame(value, position, &self.source, self.involved[position])
            }
        };
        let Site::Part(part) = self.target.frame(frame) else {
            return passage.frame(frame).map(back);
        };
        match self.origin[part] {
            Origin::Same(other) => Some(self.target.move_frame(frame, part, &self.source, other)),
            Origin::Produced(index) => passage
                .frame(self.target.move_frame(frame, part, result, index))
                .map(back),
        }
    }
}

impl Flat {
    fn new(flow: Flow) -> Self {
        let world = flow
            .context
            .iter()
            .map(|set| match set.iter().as_slice() {
                [single] => encode(*single),
                _ => None,
            })
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
            .map(|(place, _)| part(*place).1 + 1)
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
            let single = match set.iter().as_slice() {
                [single] => Some(*single),
                _ => None,
            };
            let followed = single.filter(|&source| passage.propose(place, source));
            if followed.is_none() {
                passage.resource.push((place, set));
            }
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
        let (_, id) = part(place);
        let (container, origin) = part(source);
        if rebuild(place, container, origin) != source || self.container(place) != Some(container) {
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

    pub fn resource(&self, place: Place) -> Set<Place> {
        if let Ok(position) = self
            .resource
            .binary_search_by(|(candidate, _)| candidate.cmp(&place))
        {
            return self.resource[position].1.clone();
        }
        let container = self
            .container(place)
            .expect("a place the flow keeps has a container");
        let id = decode(self.token[part(place).1]).expect("a place the flow keeps has a token");
        Set::single(rebuild(place, container, id))
    }

    pub fn context(&self, world: usize) -> Set<usize> {
        if let Ok(position) = self
            .context
            .binary_search_by(|(candidate, _)| candidate.cmp(&world))
        {
            return self.context[position].1.clone();
        }
        Set::single(decode(self.world[world]).expect("a world the flow keeps has a source"))
    }

    fn place(&self, place: Place) -> Option<Place> {
        if let Ok(position) = self
            .resource
            .binary_search_by(|(candidate, _)| candidate.cmp(&place))
        {
            return match self.resource[position].1.iter().as_slice() {
                [single] => Some(*single),
                _ => None,
            };
        }
        let container = self
            .container(place)
            .expect("a place the flow keeps has a container");
        let id = decode(self.token[part(place).1]).expect("a place the flow keeps has a token");
        Some(rebuild(place, container, id))
    }

    fn world(&self, world: usize) -> Option<usize> {
        if let Ok(position) = self
            .context
            .binary_search_by(|(candidate, _)| candidate.cmp(&world))
        {
            return match self.context[position].1.iter().as_slice() {
                [single] => Some(*single),
                _ => None,
            };
        }
        Some(decode(self.world[world]).expect("a world the flow keeps has a source"))
    }
}
