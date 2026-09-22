use crate::capture::Capture;
use crate::failure::Failure;
use crate::flow::Flow;
use crate::fragment::Fragment;
use crate::introduction;
use crate::occurrence;
use crate::path::Path;
use crate::structure::Value;
use crate::support;
use crate::world;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub world: world::Identity,
    pub consumed: Vec<occurrence::Identity>,
    pub value: Fragment<Value<Capture, Capture>>,
}

pub(crate) fn apply(path: &Path, request: &Request) -> Result<introduction::Event, Failure> {
    let evidence = request.value.evidence();
    let proof = evidence.proof().ok_or(Failure::Source)?;
    if !path.state().starts_with(proof.path().state())
        || !path.record().starts_with(proof.path().record())
    {
        return Err(Failure::Source);
    }
    if !support::lineage(path, request.world, proof.path().record().len())?.contains(&proof.world())
    {
        return Err(Failure::Lineage {
            source: proof.world(),
            target: request.world,
        });
    }
    path.target().history.permits(evidence.history())?;
    let mut read = BTreeSet::new();
    for (witness, expected) in evidence.qualified() {
        let support = support::resolve(path, request.world, witness)?;
        if support.occurrence() != expected {
            return Err(Failure::Identity(expected.identity));
        }
        read.extend(support.read());
    }
    let mut target = path.target().clone();
    let mut flow = Flow::identity(path.target());
    let materialization =
        crate::materialization::include(path, evidence.capture(), &mut target, &mut flow)?;
    let value = crate::activation::rename(request.value.value().clone(), &|capture| {
        Ok(materialization.frame[&capture])
    })?;
    let context = evidence
        .capture()
        .iter()
        .map(|capture| materialization.frame[capture])
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
    event.archive = materialization.origin;
    Ok(event)
}
