use crate::basis::Set;
use crate::state::State;
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Place {
    World(usize, usize),
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
    pub world: BTreeSet<usize>,
    pub footprint: BTreeSet<Place>,
    pub exact: BTreeSet<Place>,
    pub read: BTreeSet<Place>,
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

    pub(crate) fn compose(&self, event: &Flow) -> Self {
        Self {
            resource: event
                .resource
                .iter()
                .map(|(&place, source)| {
                    (
                        place,
                        source
                            .iter()
                            .flat_map(|key| self.resource[key].iter().copied())
                            .collect(),
                    )
                })
                .collect(),
            context: event
                .context
                .iter()
                .map(|value| {
                    value
                        .iter()
                        .flat_map(|&index| self.context[index].iter().copied())
                        .collect()
                })
                .collect(),
            frame: event
                .frame
                .iter()
                .map(|index| index.and_then(|index| self.frame[index]))
                .collect(),
        }
    }

    pub(crate) fn project(
        &self,
        source: &State,
        target: &State,
        selection: &[(usize, Vec<usize>)],
        frame: usize,
    ) -> Option<Binding> {
        let mut footprint = BTreeSet::new();
        let mut exact = BTreeSet::new();
        let mut world = BTreeSet::new();
        for (index, selected) in selection {
            world.extend(&self.context[*index]);
            for &id in selected {
                let token = target.world[*index]
                    .particle
                    .iter()
                    .find(|token| token.id == id)
                    .unwrap();
                let basis = &self.resource[&Place::World(*index, id)];
                footprint.extend(basis);
                if basis.len() != 1 {
                    continue;
                }
                if let Some(&Place::World(index, id)) = basis.first()
                    && source.world[index].particle.iter().any(|value| {
                        value.id == id
                            && value.value == token.value
                            && value.capture
                                == token.capture.and_then(|capture| self.frame[capture])
                    })
                {
                    exact.insert(Place::World(index, id));
                }
            }
        }
        for place in &footprint {
            match place {
                Place::World(index, _) => {
                    world.insert(*index);
                }
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
            world,
            footprint,
            exact,
            read: BTreeSet::new(),
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
            flow: Flow {
                resource,
                context,
                frame,
            },
        }
    }
}
