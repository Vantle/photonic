use crate::basis::Set;
use crate::flow::Flow;
use crate::place::Place;
use std::num::NonZeroU32;

// An event's flow mostly renames: a surviving token keeps its place up to the renaming of its
// world, frame and id. The passage keeps those renamings and only the places and worlds that do
// not follow them, a small fraction of a set for every place, and answers every lookup exactly as
// the flow would.
pub(super) struct Passage {
    pub frame: Vec<Option<usize>>,
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

impl Passage {
    pub fn new(flow: Flow) -> Self {
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
}
