use crate::capture;
use crate::configuration::Configuration;
use crate::context;
use crate::failure::Failure;
use crate::flow::Flow;
use crate::occurrence;
use crate::path::Path;
use crate::support::Address;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Origin {
    pub address: Address,
    pub context: context::Identity,
    pub resource: BTreeMap<occurrence::Identity, occurrence::Identity>,
}

pub(crate) struct Archive {
    pub frame: BTreeMap<context::Identity, context::Identity>,
    pub origin: BTreeMap<context::Identity, Origin>,
}

pub(crate) fn restore(
    path: &Path,
    context: &BTreeSet<context::Identity>,
    target: &mut Configuration,
    flow: &mut Flow,
) -> Result<Archive, Failure> {
    let mut source = BTreeMap::new();
    for &identity in context {
        let state = path
            .state()
            .iter()
            .rposition(|state| state.frame.contains_key(&identity))
            .ok_or(Failure::Context(identity))?;
        source.insert(
            identity,
            capture::resolve(
                path,
                &Address {
                    derivation: vec![],
                    state,
                },
                identity,
            )?,
        );
    }
    let materialization =
        crate::materialization::include(path, &source.values().cloned().collect(), target, flow)?;
    Ok(Archive {
        frame: source
            .into_iter()
            .map(|(identity, capture)| (identity, materialization.frame[&capture]))
            .collect(),
        origin: materialization.origin,
    })
}
