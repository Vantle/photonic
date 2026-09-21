use crate::archive::Origin;
use crate::context;
use crate::failure::Failure;
use crate::flow::{Flow, Place};
use crate::path::Path;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn compose(
    path: &Path,
    flow: &Flow,
    archive: &BTreeMap<context::Identity, Origin>,
) -> Result<Flow, Failure> {
    let mut result = path.flow().compose(flow).map_err(Failure::Flow)?;
    for (&identity, origin) in archive {
        let historical = path.prefix(origin.state)?;
        if let Some(frame) = result.frame.get_mut(&identity) {
            *frame = *historical
                .frame
                .get(&origin.context)
                .ok_or(Failure::Context(origin.context))?;
        }
        for (&source, &target) in &origin.resource {
            if let Some(basis) = result.resource.get_mut(&Place::Held(identity, target)) {
                basis.extend(
                    historical
                        .project(&BTreeSet::from([Place::Held(origin.context, source)]))
                        .map_err(Failure::Flow)?,
                );
            }
        }
    }
    Ok(result)
}
