use crate::allocation::take;
use crate::application::Event;
use crate::configuration::Configuration;
use crate::context;
use crate::failure::Failure;
use crate::flow::{Flow, Place};
use crate::occurrence::{self, Occurrence};
use crate::structure::Rule;
use crate::world::{self, World};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Request<'a> {
    pub source: &'a Configuration,
    pub rule: &'a Rule,
    pub frame: context::Identity,
    pub owner: context::Identity,
    pub world: &'a BTreeSet<world::Identity>,
    pub footprint: &'a BTreeSet<Place>,
    pub exact: &'a BTreeSet<Place>,
    pub read: BTreeSet<Place>,
}

pub(crate) fn apply(
    request: Request<'_>,
    mut target: Configuration,
    mut flow: Flow,
) -> Result<Event, Failure> {
    let Request {
        source,
        rule,
        frame,
        owner,
        world,
        footprint,
        exact,
        read,
    } = request;
    let returning = frame == owner && source.frame[&frame].parent.is_some();
    let parent = if returning {
        source.frame[&frame].parent.unwrap()
    } else {
        frame
    };
    let consumed = if returning {
        source.frame[&frame]
            .held
            .iter()
            .map(|value| Place::Held(frame, value.identity))
            .collect::<BTreeSet<_>>()
    } else {
        BTreeSet::new()
    };
    let basis = footprint.union(&consumed).copied().collect::<BTreeSet<_>>();
    for identity in world {
        target.world.remove(identity);
        flow.context.remove(identity);
    }
    flow.resource.retain(|place, _| match place {
        Place::World(identity, _) => !world.contains(identity),
        Place::Held(_, _) => true,
    });
    for output in rule.output.destination() {
        let selected = if output.body.is_some() {
            exact
        } else {
            footprint
        };
        let removed = selected
            .iter()
            .map(|place| match place {
                Place::World(_, identity) | Place::Held(_, identity) => *identity,
            })
            .collect::<BTreeSet<_>>();
        let mut remainder = BTreeMap::new();
        for &identity in world {
            for value in &source.world[&identity].occurrence {
                if removed.contains(&value.identity) {
                    continue;
                }
                let entry = remainder
                    .entry(value.identity)
                    .or_insert_with(|| (value.clone(), BTreeSet::new()));
                entry.1.insert(Place::World(identity, value.identity));
            }
        }
        let destination = world::Identity(take(&mut target.allocation.world)?);
        let frame = if let Some(body) = &output.body {
            let identity = context::Identity(take(&mut target.allocation.context)?);
            let mut held = BTreeMap::new();
            for &place in exact.union(&consumed) {
                let value = source.occurrence(place);
                let entry = held
                    .entry(value.identity)
                    .or_insert_with(|| (value.clone(), BTreeSet::new()));
                entry.1.insert(place);
            }
            target.frame.insert(
                identity,
                context::Frame {
                    identity,
                    parent: Some(parent),
                    lexical: Some(body.context()),
                    declaration: body.activate(identity)?,
                    held: held.values().map(|(value, _)| value.clone()).collect(),
                },
            );
            flow.frame.insert(identity, None);
            for (resource, (_, basis)) in held {
                flow.resource.insert(Place::Held(identity, resource), basis);
            }
            identity
        } else {
            parent
        };
        let mut occurrence = Vec::new();
        for (identity, (value, basis)) in remainder {
            occurrence.push(value);
            flow.resource
                .insert(Place::World(destination, identity), basis);
        }
        for value in output.particle.value() {
            let identity = occurrence::Identity(take(&mut target.allocation.occurrence)?);
            occurrence.push(Occurrence {
                identity,
                value: value.clone(),
                history: source.history.clone(),
            });
            flow.resource
                .insert(Place::World(destination, identity), basis.clone());
        }
        target.world.insert(
            destination,
            World {
                identity: destination,
                context: frame,
                occurrence,
            },
        );
        flow.context.insert(destination, world.clone());
    }
    target.reclaim();
    flow.frame
        .retain(|identity, _| target.frame.contains_key(identity));
    flow.resource.retain(|place, _| match place {
        Place::Held(identity, _) => target.frame.contains_key(identity),
        Place::World(_, _) => true,
    });
    flow.validate(source, &target).map_err(Failure::Flow)?;
    Ok(Event {
        target,
        flow,
        read,
        consumed: footprint.clone(),
        owner,
        archive: BTreeMap::new(),
    })
}
