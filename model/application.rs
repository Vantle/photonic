use crate::configuration::Configuration;
use crate::context;
use crate::failure::Failure;
use crate::flow::{Flow, Place};
use crate::occurrence;
use crate::structure::{Particle, Rule, Value};
use crate::world::{self, World};
use std::collections::BTreeSet;

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
pub struct Event {
    pub target: Configuration,
    pub flow: Flow,
    pub read: BTreeSet<Place>,
    pub consumed: BTreeSet<Place>,
    pub owner: context::Identity,
}

pub(crate) fn select(
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

pub(crate) fn code<'a>(
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

pub(crate) fn binding(
    state: &Configuration,
    request: &Request,
    input: &crate::structure::Input,
) -> Result<BTreeSet<Place>, Failure> {
    let expected = input.particle().len().max(1);
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
            input.particle().get(position).unwrap_or(&empty),
        )?);
    }
    Ok(consumed)
}

pub fn apply(state: &Configuration, request: &Request) -> Result<Event, Failure> {
    let (site, selected) = select(state, request)?;
    let (rule, owner, read) = code(state, request.code, site, &selected)?;
    let consumed = binding(state, request, &rule.input)?;
    crate::rewrite::apply(
        crate::rewrite::Request {
            source: state,
            rule,
            frame: site,
            owner,
            world: &selected,
            footprint: &consumed,
            exact: &consumed,
            read,
        },
        state.clone(),
        Flow::identity(state),
    )
}
