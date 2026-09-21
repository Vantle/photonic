use crate::configuration::Configuration;
use crate::{context, occurrence, world};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Place {
    World(world::Identity, occurrence::Identity),
    Held(context::Identity, occurrence::Identity),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Endpoint {
    Source,
    Target,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Failure {
    Resource {
        endpoint: Endpoint,
        place: Place,
    },
    World {
        endpoint: Endpoint,
        identity: world::Identity,
    },
    Frame {
        endpoint: Endpoint,
        identity: context::Identity,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Flow {
    pub resource: BTreeMap<Place, BTreeSet<Place>>,
    pub context: BTreeMap<world::Identity, BTreeSet<world::Identity>>,
    pub frame: BTreeMap<context::Identity, Option<context::Identity>>,
}

fn resource(state: &Configuration) -> BTreeSet<Place> {
    state
        .world()
        .flat_map(|world| {
            world
                .occurrence
                .iter()
                .map(|value| Place::World(world.identity, value.identity))
        })
        .chain(state.frame().flat_map(|frame| {
            frame
                .held
                .iter()
                .map(|value| Place::Held(frame.identity, value.identity))
        }))
        .collect()
}

impl Flow {
    pub fn identity(state: &Configuration) -> Self {
        Self {
            resource: resource(state)
                .into_iter()
                .map(|place| (place, BTreeSet::from([place])))
                .collect(),
            context: state
                .world()
                .map(|world| (world.identity, BTreeSet::from([world.identity])))
                .collect(),
            frame: state
                .frame()
                .map(|frame| (frame.identity, Some(frame.identity)))
                .collect(),
        }
    }

    pub fn validate(&self, source: &Configuration, target: &Configuration) -> Result<(), Failure> {
        let expected = resource(target);
        let actual = self.resource.keys().copied().collect();
        if let Some(&place) = expected.symmetric_difference(&actual).next() {
            return Err(Failure::Resource {
                endpoint: Endpoint::Target,
                place,
            });
        }
        let available = resource(source);
        for &place in self.resource.values().flatten() {
            if !available.contains(&place) {
                return Err(Failure::Resource {
                    endpoint: Endpoint::Source,
                    place,
                });
            }
        }
        let expected = target
            .world()
            .map(|world| world.identity)
            .collect::<BTreeSet<_>>();
        let actual = self.context.keys().copied().collect();
        if let Some(&identity) = expected.symmetric_difference(&actual).next() {
            return Err(Failure::World {
                endpoint: Endpoint::Target,
                identity,
            });
        }
        for &identity in self.context.values().flatten() {
            if !source.world.contains_key(&identity) {
                return Err(Failure::World {
                    endpoint: Endpoint::Source,
                    identity,
                });
            }
        }
        let expected = target
            .frame()
            .map(|frame| frame.identity)
            .collect::<BTreeSet<_>>();
        let actual = self.frame.keys().copied().collect();
        if let Some(&identity) = expected.symmetric_difference(&actual).next() {
            return Err(Failure::Frame {
                endpoint: Endpoint::Target,
                identity,
            });
        }
        for &identity in self.frame.values().flatten() {
            if !source.frame.contains_key(&identity) {
                return Err(Failure::Frame {
                    endpoint: Endpoint::Source,
                    identity,
                });
            }
        }
        Ok(())
    }

    pub fn project(&self, source: &BTreeSet<Place>) -> Result<BTreeSet<Place>, Failure> {
        let mut result = BTreeSet::new();
        for &place in source {
            result.extend(self.resource.get(&place).ok_or(Failure::Resource {
                endpoint: Endpoint::Target,
                place,
            })?);
        }
        Ok(result)
    }

    pub fn compose(&self, next: &Self) -> Result<Self, Failure> {
        let resource = next
            .resource
            .iter()
            .map(|(&place, source)| Ok((place, self.project(source)?)))
            .collect::<Result<_, _>>()?;
        let mut context = BTreeMap::new();
        for (&identity, source) in &next.context {
            let mut result = BTreeSet::new();
            for &identity in source {
                result.extend(self.context.get(&identity).ok_or(Failure::World {
                    endpoint: Endpoint::Target,
                    identity,
                })?);
            }
            context.insert(identity, result);
        }
        let frame = next
            .frame
            .iter()
            .map(|(&identity, source)| {
                let result = match *source {
                    None => None,
                    Some(source) => *self.frame.get(&source).ok_or(Failure::Frame {
                        endpoint: Endpoint::Target,
                        identity: source,
                    })?,
                };
                Ok((identity, result))
            })
            .collect::<Result<_, Failure>>()?;
        Ok(Self {
            resource,
            context,
            frame,
        })
    }
}
