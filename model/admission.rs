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
    pub place: Place,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub world: world::Identity,
    pub consumed: Vec<occurrence::Identity>,
    pub value: Fragment<Value>,
    pub witness: Vec<Witness>,
}

pub(crate) fn apply(path: &Path, request: &Request) -> Result<introduction::Event, Failure> {
    path.target()
        .history
        .permits(request.value.evidence().history())?;
    let mut supplied = BTreeMap::new();
    for &witness in &request.witness {
        let identity = witness.place.occurrence();
        if supplied.insert(identity, witness).is_some() {
            return Err(Failure::Witness(identity));
        }
    }
    let mut read = BTreeSet::new();
    for expected in request.value.evidence().witness() {
        let witness = supplied
            .remove(&expected.identity)
            .ok_or(Failure::Witness(expected.identity))?;
        let location = crate::support::Request {
            address: crate::support::Address {
                derivation: vec![],
                state: witness.state,
            },
            world: witness.world,
            place: witness.place,
        };
        let support = crate::support::resolve(path, request.world, &location)?;
        if support.occurrence() != expected {
            return Err(Failure::Identity(expected.identity));
        }
        read.extend(support.read());
    }
    if let Some((&identity, _)) = supplied.first_key_value() {
        return Err(Failure::Witness(identity));
    }
    let mut target = path.target().clone();
    let mut flow = crate::flow::Flow::identity(path.target());
    let archive = crate::archive::restore(
        path,
        request.value.evidence().context(),
        &mut target,
        &mut flow,
    )?;
    let value = crate::activation::rename(request.value.value().clone(), &|identity| {
        Ok(archive.frame[&identity])
    })?;
    let context = request
        .value
        .evidence()
        .context()
        .iter()
        .map(|identity| archive.frame[identity])
        .collect();
    let mut event = introduction::publish(
        introduction::Request {
            state: path.target(),
            world: request.world,
            consumed: &request.consumed,
            value: &value,
            context: &context,
            read,
        },
        target,
        flow,
    )?;
    event.archive = archive.origin;
    Ok(event)
}
