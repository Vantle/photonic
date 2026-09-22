use crate::bijection;
use crate::configuration::Configuration;
use crate::context;
use crate::occurrence::{self, Occurrence};
use crate::shape::Shape;
use crate::world;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mapping {
    frame: BTreeMap<context::Identity, context::Identity>,
    world: BTreeMap<world::Identity, world::Identity>,
    occurrence: BTreeMap<occurrence::Identity, occurrence::Identity>,
}

impl Mapping {
    pub fn frame(&self) -> &BTreeMap<context::Identity, context::Identity> {
        &self.frame
    }

    pub fn world(&self) -> &BTreeMap<world::Identity, world::Identity> {
        &self.world
    }

    pub fn occurrence(&self) -> &BTreeMap<occurrence::Identity, occurrence::Identity> {
        &self.occurrence
    }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum Owner {
    World(world::Identity),
    Frame(context::Identity),
}

struct Resource<'a> {
    occurrence: &'a Occurrence,
    owner: BTreeSet<Owner>,
}

fn resource(state: &Configuration) -> BTreeMap<occurrence::Identity, Resource<'_>> {
    let mut result = BTreeMap::new();
    let source = state
        .world()
        .map(|world| (Owner::World(world.identity), &world.occurrence))
        .chain(
            state
                .frame()
                .map(|frame| (Owner::Frame(frame.identity), &frame.held)),
        );
    for (owner, source) in source {
        for occurrence in source {
            result
                .entry(occurrence.identity)
                .or_insert_with(|| Resource {
                    occurrence,
                    owner: BTreeSet::new(),
                })
                .owner
                .insert(owner);
        }
    }
    result
}

fn edge(
    source: Option<context::Identity>,
    target: Option<context::Identity>,
    frame: &BTreeMap<context::Identity, context::Identity>,
) -> bool {
    match (source, target) {
        (None, None) => true,
        (Some(source), Some(target)) => frame.get(&source).is_none_or(|&mapped| mapped == target),
        _ => false,
    }
}

pub fn compare(source: &Configuration, target: &Configuration) -> Option<Mapping> {
    find(source, target, |_| true)
}

pub fn find(
    source: &Configuration,
    target: &Configuration,
    accept: impl Fn(&Mapping) -> bool,
) -> Option<Mapping> {
    if source.history() != target.history()
        || source.frame.len() != target.frame.len()
        || source.world.len() != target.world.len()
    {
        return None;
    }
    let original = resource(source);
    let destination = resource(target);
    if original.len() != destination.len() {
        return None;
    }
    let erased = source
        .frame()
        .map(|frame| (frame.identity, context::Identity(0)))
        .collect();
    let erased = source
        .frame()
        .map(|frame| {
            (
                frame.identity,
                Shape::declaration(&frame.declaration, &erased),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let collapsed = target
        .frame()
        .map(|frame| (frame.identity, context::Identity(0)))
        .collect();
    let collapsed = target
        .frame()
        .map(|frame| {
            (
                frame.identity,
                Shape::declaration(&frame.declaration, &collapsed),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let identity = target
        .frame()
        .map(|frame| (frame.identity, frame.identity))
        .collect();
    let declaration = target
        .frame()
        .map(|frame| {
            (
                frame.identity,
                Shape::declaration(&frame.declaration, &identity),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let value = destination
        .iter()
        .map(|(&resource, entry)| (resource, Shape::value(&entry.occurrence.value, &identity)))
        .collect::<BTreeMap<_, _>>();
    let candidate = source
        .frame()
        .map(|frame| {
            let candidate = target
                .frame()
                .filter(|other| {
                    (frame.identity == source.root()) == (other.identity == target.root())
                        && frame.parent.is_some() == other.parent.is_some()
                        && frame.lexical.is_some() == other.lexical.is_some()
                        && frame.held.len() == other.held.len()
                        && erased[&frame.identity] == collapsed[&other.identity]
                })
                .map(|frame| frame.identity)
                .collect();
            (frame.identity, candidate)
        })
        .collect();
    let mut result = None;
    bijection::search(
        &candidate,
        BTreeMap::new(),
        &|mapping| {
            mapping.iter().all(|(left, right)| {
                let left = &source.frame[left];
                let right = &target.frame[right];
                edge(left.parent, right.parent, mapping)
                    && edge(left.lexical, right.lexical, mapping)
            })
        },
        &mut |frame| {
            if frame.iter().any(|(left, right)| {
                Shape::declaration(&source.frame[left].declaration, frame) != declaration[right]
            }) {
                return false;
            }
            let candidate = source
                .world()
                .map(|world| {
                    (
                        world.identity,
                        target
                            .world()
                            .filter(|other| {
                                frame[&world.context] == other.context
                                    && world.occurrence.len() == other.occurrence.len()
                            })
                            .map(|world| world.identity)
                            .collect(),
                    )
                })
                .collect();
            bijection::search(&candidate, BTreeMap::new(), &|_| true, &mut |world| {
                let candidate = original
                    .iter()
                    .map(|(&identity, resource)| {
                        let shape = Shape::value(&resource.occurrence.value, frame);
                        let owner = resource
                            .owner
                            .iter()
                            .map(|owner| match owner {
                                Owner::World(identity) => Owner::World(world[identity]),
                                Owner::Frame(identity) => Owner::Frame(frame[identity]),
                            })
                            .collect::<BTreeSet<_>>();
                        (
                            identity,
                            destination
                                .iter()
                                .filter(|(identity, other)| {
                                    shape == value[identity]
                                        && resource.occurrence.history == other.occurrence.history
                                        && owner == other.owner
                                })
                                .map(|(&identity, _)| identity)
                                .collect(),
                        )
                    })
                    .collect();
                bijection::search(&candidate, BTreeMap::new(), &|_| true, &mut |occurrence| {
                    let mapping = Mapping {
                        frame: frame.clone(),
                        world: world.clone(),
                        occurrence: occurrence.clone(),
                    };
                    if !accept(&mapping) {
                        return false;
                    }
                    result = Some(mapping);
                    true
                })
            })
        },
    );
    result
}
