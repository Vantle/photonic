use crate::failure::Failure;
use crate::flow::Place;
use crate::fragment::Fragment;
use crate::introduction;
use crate::occurrence;
use crate::path::Path;
use crate::structure::Value;
use crate::world;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Witness {
    pub state: usize,
    pub world: world::Identity,
    pub occurrence: occurrence::Identity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub world: world::Identity,
    pub consumed: Vec<occurrence::Identity>,
    pub value: Fragment<Value>,
    pub witness: Vec<Witness>,
}

pub(crate) fn apply(path: &Path, request: &Request) -> Result<introduction::Event, Failure> {
    let mut supplied = BTreeMap::new();
    for &witness in &request.witness {
        if supplied.insert(witness.occurrence, witness).is_some() {
            return Err(Failure::Witness(witness.occurrence));
        }
    }
    let mut read = BTreeSet::new();
    for expected in request.value.evidence().witness() {
        let witness = supplied
            .remove(&expected.identity)
            .ok_or(Failure::Witness(expected.identity))?;
        let state = path
            .state()
            .get(witness.state)
            .ok_or(Failure::State(witness.state))?;
        let world = state
            .world
            .get(&witness.world)
            .ok_or(Failure::World(witness.world))?;
        let occurrence = world
            .occurrence
            .iter()
            .find(|value| value.identity == witness.occurrence)
            .ok_or(Failure::Occurrence(witness.occurrence))?;
        if occurrence != expected {
            return Err(Failure::Identity(witness.occurrence));
        }
        let mut lineage = BTreeSet::from([request.world]);
        for record in path.record()[witness.state..].iter().rev() {
            let mut previous = BTreeSet::new();
            for identity in lineage {
                previous.extend(
                    record
                        .flow
                        .context
                        .get(&identity)
                        .ok_or(Failure::World(identity))?,
                );
            }
            lineage = previous;
        }
        if !lineage.contains(&witness.world) {
            return Err(Failure::Lineage {
                source: witness.world,
                target: request.world,
            });
        }
        if witness.state == path.record().len() {
            read.insert(Place::World(witness.world, witness.occurrence));
        }
    }
    if let Some((&identity, _)) = supplied.first_key_value() {
        return Err(Failure::Witness(identity));
    }
    introduction::publish(
        path.target(),
        request.world,
        &request.consumed,
        &request.value,
        read,
    )
}
