use crate::basis::Set;
use crate::state::State;
use serde::Serialize;
use std::collections::BTreeSet;

mod composition;
mod key;
mod store;
mod template;
mod union;

pub(crate) use store::Store;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Place {
    World(usize, usize),
    Context(usize, usize),
    Held(usize, usize),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Flow {
    pub resource: crate::relation::Map<Place, Set<Place>>,
    pub context: Vec<Set<usize>>,
    pub frame: Vec<Option<usize>>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Binding {
    pub world: Set<usize>,
    pub footprint: Set<Place>,
    pub exact: Set<Place>,
    pub read: Set<Place>,
}

pub(crate) struct Closure<'state> {
    pub state: &'state State,
    pub flow: &'state Flow,
    pub capture: usize,
}

pub(crate) struct Applied {
    pub state: State,
    pub flow: Flow,
}

impl Flow {
    pub(crate) fn identity(state: &State) -> Self {
        let mut resource = Vec::new();
        for (world, value) in state.world.iter().enumerate() {
            for token in &value.particle {
                let place = Place::World(world, token.id);
                resource.push((place, Set::single(place)));
            }
        }
        for (frame, value) in state.frame.iter().enumerate() {
            for token in &value.particle {
                let place = Place::Context(frame, token.id);
                resource.push((place, Set::single(place)));
            }
            for token in &value.held {
                let place = Place::Held(frame, token.id);
                resource.push((place, Set::single(place)));
            }
        }
        Self {
            resource: resource.into_iter().collect(),
            context: (0..state.world.len()).map(Set::single).collect(),
            frame: (0..state.frame.len()).map(Some).collect(),
        }
    }

    pub(crate) fn compose(&self, event: &Self) -> Self {
        composition::Composition::default().compose(self, event)
    }

    pub(crate) fn project(
        &self,
        source: &State,
        target: &State,
        selection: &[crate::slot::Slot],
        frame: usize,
    ) -> Option<Binding> {
        let mut footprint = BTreeSet::new();
        let mut exact = BTreeSet::new();
        let mut world = BTreeSet::new();
        let mut selected = BTreeSet::new();
        for slot in selection {
            if let Some(index) = slot.location.world() {
                world.extend(&self.context[index]);
            }
            for &id in &slot.token {
                let place = target.resolve(slot.location, id)?;
                if !selected.insert(place) {
                    return None;
                }
                let token = target.token(place)?;
                let basis = &self.resource[&place];
                footprint.extend(basis);
                if basis.len() != 1 {
                    continue;
                }
                let place = *basis.first()?;
                if source.token(place).is_some_and(|value| {
                    value.value == token.value
                        && value.capture == token.capture.and_then(|capture| self.frame[capture])
                }) {
                    exact.insert(place);
                }
            }
        }
        for place in &footprint {
            match place {
                Place::World(index, _) => {
                    world.insert(*index);
                }
                Place::Context(_, _) => {}
                Place::Held(_, _) => return None,
            }
        }
        if world
            .iter()
            .any(|&index| source.world[index].frame != frame)
        {
            return None;
        }
        Some(Binding {
            world: world.into(),
            footprint: footprint.into(),
            exact: exact.into(),
            read: Set::default(),
        })
    }
}

impl Applied {
    #[cfg(test)]
    pub(crate) fn canonical(self) -> Self {
        let canonical = self.state.canonical();
        self.flow.rename(canonical)
    }
}

impl Flow {
    pub(crate) fn rename(self, canonical: crate::state::Canonical) -> Applied {
        let resource = self
            .resource
            .into_iter()
            .filter_map(|(place, basis)| {
                let place = match place {
                    Place::World(index, id) => {
                        Place::World(canonical.world[index]?, canonical.resource[&id])
                    }
                    Place::Context(index, id) => {
                        Place::Context(canonical.frame[index]?, canonical.resource[&id])
                    }
                    Place::Held(index, id) => {
                        Place::Held(canonical.frame[index]?, canonical.resource[&id])
                    }
                };
                Some((place, basis))
            })
            .collect();
        let mut context = vec![Set::default(); canonical.state.world.len()];
        for (index, target) in canonical.world.iter().enumerate() {
            if let Some(target) = target {
                context[*target] = self.context[index].clone();
            }
        }
        let mut frame = vec![None; canonical.state.frame.len()];
        for (index, target) in canonical.frame.iter().enumerate() {
            if let Some(target) = target {
                frame[*target] = self.frame[index];
            }
        }
        Applied {
            state: canonical.state,
            flow: Self {
                resource,
                context,
                frame,
            },
        }
    }
}

#[cfg(test)]
#[path = "test/flow.rs"]
mod test;

impl Binding {
    pub(crate) fn select(state: &State, selection: &[crate::slot::Slot]) -> Option<Self> {
        let mut footprint = BTreeSet::new();
        for slot in selection {
            for &id in &slot.token {
                let place = state.resolve(slot.location, id)?;
                if !footprint.insert(place) {
                    return None;
                }
            }
        }
        let footprint = Set::from(footprint);
        Some(Self {
            world: selection
                .iter()
                .filter_map(|slot| slot.location.world())
                .collect(),
            exact: footprint.clone(),
            footprint,
            read: Set::default(),
        })
    }
}
