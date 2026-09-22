use crate::context;
use crate::failure::Failure;
use crate::path::Path;
use crate::support::{self, Address};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Capture {
    address: Address,
    identity: context::Identity,
}

impl Capture {
    pub fn address(&self) -> &Address {
        &self.address
    }

    pub fn identity(&self) -> context::Identity {
        self.identity
    }
}

pub(crate) fn resolve(
    path: &Path,
    address: &Address,
    identity: context::Identity,
) -> Result<Capture, Failure> {
    let mut address = address.clone();
    let mut identity = identity;
    loop {
        let cursor = support::descend(path, &address.derivation)?;
        let state = cursor
            .state()
            .get(address.state)
            .ok_or(Failure::State(address.state))?;
        if !state.frame.contains_key(&identity) {
            return Err(Failure::Context(identity));
        }
        let position = cursor.state()[..=address.state]
            .iter()
            .position(|state| state.frame.contains_key(&identity))
            .unwrap();
        if position == 0 {
            if let Some(anchor) = address.derivation.pop() {
                address.state = anchor;
                continue;
            }
            address.state = 0;
            return Ok(Capture { address, identity });
        }
        if let Some(origin) = cursor.record()[position - 1].archive.get(&identity) {
            address
                .derivation
                .extend_from_slice(&origin.address.derivation);
            address.state = origin.address.state;
            identity = origin.context;
            continue;
        }
        address.state = position;
        return Ok(Capture { address, identity });
    }
}
