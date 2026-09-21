use crate::allocation::take;
use crate::configuration::Configuration;
use crate::failure::Failure;
use crate::flow::{Flow, Place};
use crate::fragment::Fragment;
use crate::occurrence::{self, Occurrence};
use crate::structure::Value;
use crate::world::{self, World};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    pub target: Configuration,
    pub flow: Flow,
    pub read: BTreeSet<Place>,
    pub consumed: BTreeSet<Place>,
    pub context: BTreeSet<crate::context::Identity>,
    pub occurrence: occurrence::Identity,
}

pub fn apply(
    state: &Configuration,
    world: world::Identity,
    consumed: &[occurrence::Identity],
    value: &Fragment<Value>,
) -> Result<Event, Failure> {
    let source = state.world.get(&world).ok_or(Failure::World(world))?;
    let mut read = BTreeSet::new();
    for witness in value.evidence().witness() {
        let occurrence = source
            .occurrence
            .iter()
            .find(|value| value.identity == witness.identity)
            .ok_or(Failure::Occurrence(witness.identity))?;
        if occurrence != witness {
            return Err(Failure::Identity(witness.identity));
        }
        read.insert(Place::World(world, witness.identity));
    }
    publish(state, world, consumed, value, read)
}

pub(crate) fn publish(
    state: &Configuration,
    world: world::Identity,
    consumed: &[occurrence::Identity],
    value: &Fragment<Value>,
    read: BTreeSet<Place>,
) -> Result<Event, Failure> {
    let source = state.world.get(&world).ok_or(Failure::World(world))?;
    state.history.permits(value.evidence().history())?;
    for &identity in value.evidence().context() {
        if !state.frame.contains_key(&identity) {
            return Err(Failure::Context(identity));
        }
    }
    let mut selected = BTreeSet::new();
    for &identity in consumed {
        if !selected.insert(identity) {
            return Err(Failure::Repeated(identity));
        }
        if !source
            .occurrence
            .iter()
            .any(|value| value.identity == identity)
        {
            return Err(Failure::Occurrence(identity));
        }
    }
    let consumed = selected
        .iter()
        .map(|&identity| Place::World(world, identity))
        .collect::<BTreeSet<_>>();
    let mut target = state.clone();
    let destination = world::Identity(take(&mut target.allocation.world)?);
    let identity = occurrence::Identity(take(&mut target.allocation.occurrence)?);
    let mut flow = Flow::identity(state);
    flow.context.remove(&world);
    flow.context.insert(destination, BTreeSet::from([world]));
    let mut occurrence = Vec::new();
    for value in &source.occurrence {
        let place = Place::World(world, value.identity);
        flow.resource.remove(&place);
        if selected.contains(&value.identity) {
            continue;
        }
        occurrence.push(value.clone());
        flow.resource.insert(
            Place::World(destination, value.identity),
            BTreeSet::from([place]),
        );
    }
    occurrence.push(Occurrence {
        identity,
        value: value.value().clone(),
        history: state.history.clone(),
    });
    flow.resource
        .insert(Place::World(destination, identity), consumed.clone());
    target.world.remove(&world);
    target.world.insert(
        destination,
        World {
            identity: destination,
            context: source.context,
            occurrence,
        },
    );
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
        context: value.evidence().context().clone(),
        occurrence: identity,
    })
}
