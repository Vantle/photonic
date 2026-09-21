use crate::allocation::take;
use crate::configuration::Configuration;
use crate::context;
use crate::failure::Failure;
use crate::occurrence::{self, Occurrence};
use crate::structure::{Particle, Rule, Value};
use crate::world::{self, World};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Place {
    World(world::Identity, occurrence::Identity),
    Held(context::Identity, occurrence::Identity),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Code {
    Local {
        world: world::Identity,
        occurrence: occurrence::Identity,
    },
    Declaration {
        context: context::Identity,
        position: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selection {
    pub world: world::Identity,
    pub occurrence: Vec<occurrence::Identity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub code: Code,
    pub selection: Vec<Selection>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Flow {
    pub resource: BTreeMap<Place, BTreeSet<Place>>,
    pub context: BTreeMap<world::Identity, BTreeSet<world::Identity>>,
    pub frame: BTreeMap<context::Identity, Option<context::Identity>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    pub target: Configuration,
    pub flow: Flow,
    pub read: BTreeSet<Place>,
    pub consumed: BTreeSet<Place>,
    pub owner: context::Identity,
}

fn select(
    state: &Configuration,
    request: &Request,
) -> Result<(context::Identity, BTreeSet<world::Identity>), Failure> {
    let mut selected = BTreeSet::new();
    let mut context = None;
    for selection in &request.selection {
        let world = state
            .world
            .get(&selection.world)
            .ok_or(Failure::World(selection.world))?;
        if !selected.insert(selection.world) {
            return Err(Failure::World(selection.world));
        }
        if context.is_some_and(|context| world.context != context) {
            return Err(Failure::Context(world.context));
        }
        context = Some(world.context);
    }
    Ok((context.ok_or(Failure::Site)?, selected))
}

fn code<'a>(
    state: &'a Configuration,
    source: Code,
    site: context::Identity,
    selected: &BTreeSet<world::Identity>,
) -> Result<(&'a Rule, context::Identity, BTreeSet<Place>), Failure> {
    match source {
        Code::Local { world, occurrence } => {
            if !selected.contains(&world) {
                return Err(Failure::World(world));
            }
            let source = state.world[&world]
                .occurrence
                .iter()
                .find(|value| value.identity == occurrence)
                .ok_or(Failure::Occurrence(occurrence))?;
            let Value::Rule(rule) = &source.value else {
                return Err(Failure::Rule);
            };
            Ok((
                rule,
                rule.context,
                BTreeSet::from([Place::World(world, occurrence)]),
            ))
        }
        Code::Declaration { context, position } => {
            if !state.visible(site, context) {
                return Err(Failure::Context(context));
            }
            let rule = state.frame[&context]
                .declaration
                .get(position)
                .ok_or(Failure::Declaration { context, position })?;
            Ok((rule, context, BTreeSet::new()))
        }
    }
}

fn matching(
    world: &World,
    selected: &[occurrence::Identity],
    particle: &Particle,
) -> Result<BTreeSet<Place>, Failure> {
    if selected.len() != particle.value().len() {
        return Err(Failure::Arity {
            expected: particle.value().len(),
            actual: selected.len(),
        });
    }
    let mut unique = BTreeSet::new();
    let mut value = Vec::new();
    for &identity in selected {
        if !unique.insert(identity) {
            return Err(Failure::Repeated(identity));
        }
        let source = world
            .occurrence
            .iter()
            .find(|value| value.identity == identity)
            .ok_or(Failure::Occurrence(identity))?;
        value.push(source.value.clone());
    }
    if Particle::new(value) != *particle {
        return Err(Failure::Match(world.identity));
    }
    Ok(unique
        .into_iter()
        .map(|identity| Place::World(world.identity, identity))
        .collect())
}

pub fn apply(state: &Configuration, request: &Request) -> Result<Event, Failure> {
    let (site, selected) = select(state, request)?;
    let (rule, owner, read) = code(state, request.code, site, &selected)?;
    let expected = rule.input.particle().len().max(1);
    if request.selection.len() != expected {
        return Err(Failure::Arity {
            expected,
            actual: request.selection.len(),
        });
    }
    let mut consumed = BTreeSet::new();
    let empty = Particle::default();
    for (position, selection) in request.selection.iter().enumerate() {
        consumed.extend(matching(
            &state.world[&selection.world],
            &selection.occurrence,
            rule.input.particle().get(position).unwrap_or(&empty),
        )?);
    }
    let frame = &state.frame[&site];
    let returning = site == owner && frame.parent.is_some();
    let parent = if returning {
        frame.parent.unwrap()
    } else {
        site
    };
    let mut basis = consumed.clone();
    let mut held = BTreeMap::new();
    if returning {
        for value in &frame.held {
            let place = Place::Held(site, value.identity);
            basis.insert(place);
            held.insert(value.identity, (value.clone(), BTreeSet::from([place])));
        }
    }
    let removed = consumed
        .iter()
        .map(|place| match place {
            Place::World(_, identity) | Place::Held(_, identity) => *identity,
        })
        .collect::<BTreeSet<_>>();
    let mut remainder = BTreeMap::new();
    for &identity in &selected {
        for value in &state.world[&identity].occurrence {
            let place = Place::World(identity, value.identity);
            let target = if removed.contains(&value.identity) {
                &mut held
            } else {
                &mut remainder
            };
            let entry = target
                .entry(value.identity)
                .or_insert_with(|| (value.clone(), BTreeSet::new()));
            if !removed.contains(&value.identity) || consumed.contains(&place) {
                entry.1.insert(place);
            }
        }
    }
    let mut target = state.clone();
    let mut flow = Flow {
        resource: BTreeMap::new(),
        context: BTreeMap::new(),
        frame: state
            .frame
            .keys()
            .map(|&identity| (identity, Some(identity)))
            .collect(),
    };
    for &identity in &selected {
        target.world.remove(&identity);
    }
    for value in target.world.values() {
        flow.context
            .insert(value.identity, BTreeSet::from([value.identity]));
        for occurrence in &value.occurrence {
            let place = Place::World(value.identity, occurrence.identity);
            flow.resource.insert(place, BTreeSet::from([place]));
        }
    }
    for frame in target.frame.values() {
        for occurrence in &frame.held {
            let place = Place::Held(frame.identity, occurrence.identity);
            flow.resource.insert(place, BTreeSet::from([place]));
        }
    }
    for output in rule.output.destination() {
        let destination = world::Identity(take(&mut target.allocation.world)?);
        let frame = if let Some(body) = &output.body {
            let identity = context::Identity(take(&mut target.allocation.context)?);
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
            for (&resource, (_, source)) in &held {
                flow.resource
                    .insert(Place::Held(identity, resource), source.clone());
            }
            identity
        } else {
            parent
        };
        let mut occurrence = Vec::new();
        for (&identity, (value, source)) in &remainder {
            occurrence.push(value.clone());
            flow.resource
                .insert(Place::World(destination, identity), source.clone());
        }
        for value in output.particle.value() {
            let identity = occurrence::Identity(take(&mut target.allocation.occurrence)?);
            occurrence.push(Occurrence {
                identity,
                value: value.clone(),
                history: state.history.clone(),
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
        flow.context.insert(destination, selected.clone());
    }
    target.reclaim();
    flow.frame
        .retain(|identity, _| target.frame.contains_key(identity));
    flow.resource.retain(|place, _| match place {
        Place::Held(identity, _) => target.frame.contains_key(identity),
        Place::World(_, _) => true,
    });
    Ok(Event {
        target,
        flow,
        read,
        consumed,
        owner,
    })
}
