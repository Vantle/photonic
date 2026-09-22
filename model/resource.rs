use crate::configuration::Configuration;
use crate::failure::Failure;
use crate::occurrence;
use crate::path::{Path, Step};
use crate::support::{self, Address};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Resource {
    address: Address,
    identity: occurrence::Identity,
}

pub(crate) struct Registry {
    edge: BTreeMap<Resource, BTreeSet<Resource>>,
}

fn contains(state: &Configuration, identity: occurrence::Identity) -> bool {
    state
        .world()
        .flat_map(|world| &world.occurrence)
        .chain(state.frame().flat_map(|frame| &frame.held))
        .any(|value| value.identity == identity)
}

fn nominal(
    path: &Path,
    address: &Address,
    identity: occurrence::Identity,
) -> Result<Resource, Failure> {
    let mut address = address.clone();
    loop {
        let cursor = support::descend(path, &address.derivation)?;
        let state = cursor
            .state()
            .get(address.state)
            .ok_or(Failure::State(address.state))?;
        if !contains(state, identity) {
            return Err(Failure::Occurrence(identity));
        }
        let position = cursor.state()[..=address.state]
            .iter()
            .position(|state| contains(state, identity))
            .unwrap();
        if position == 0
            && let Some(anchor) = address.derivation.pop()
        {
            address.state = anchor;
            continue;
        }
        address.state = position;
        return Ok(Resource { address, identity });
    }
}

impl Registry {
    pub fn new(path: &Path) -> Result<Self, Failure> {
        let mut result = Self {
            edge: BTreeMap::new(),
        };
        let mut pending = vec![(path, Vec::new())];
        while let Some((cursor, derivation)) = pending.pop() {
            for (position, record) in cursor.record().iter().enumerate() {
                let target = Address {
                    derivation: derivation.clone(),
                    state: position + 1,
                };
                for origin in record.archive.values() {
                    let mut source = origin.address.clone();
                    source.derivation.splice(..0, derivation.iter().copied());
                    for (&original, &copied) in &origin.resource {
                        if !contains(&cursor.state()[position + 1], copied) {
                            continue;
                        }
                        let source = nominal(path, &source, original)?;
                        let target = nominal(path, &target, copied)?;
                        result
                            .edge
                            .entry(source.clone())
                            .or_default()
                            .insert(target.clone());
                        result.edge.entry(target).or_default().insert(source);
                    }
                }
                if let Step::Inference { path: child, .. } = &record.step {
                    let mut nested = derivation.clone();
                    nested.push(position);
                    pending.push((child, nested));
                }
            }
        }
        Ok(result)
    }

    pub fn resolve(
        &self,
        path: &Path,
        address: &Address,
        identity: occurrence::Identity,
    ) -> Result<Resource, Failure> {
        let mut pending = BTreeSet::from([nominal(path, address, identity)?]);
        let mut visited = BTreeSet::new();
        while let Some(resource) = pending.pop_first() {
            if !visited.insert(resource.clone()) {
                continue;
            }
            pending.extend(self.edge.get(&resource).into_iter().flatten().cloned());
        }
        Ok(visited.pop_first().unwrap())
    }
}
