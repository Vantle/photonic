use crate::failure::Failure;
use crate::slot::Slot;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Identity(pub u64);

pub struct Scope {
    identity: Identity,
    next: usize,
}

impl Scope {
    pub fn new(identity: Identity) -> Self {
        Self { identity, next: 0 }
    }

    pub fn identity(&self) -> Identity {
        self.identity
    }

    pub fn declare<Value>(&mut self) -> Result<Slot<Value>, Failure> {
        let position = self.next;
        self.next = position.checked_add(1).ok_or(Failure::Capacity)?;
        Ok(Slot::new(self.identity, position))
    }
}
