use crate::failure::Failure;
use crate::scope;
use crate::slot::Slot;
use crate::structure::{Body, Input, Output, Particle, Value};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sort {
    Value,
    Particle,
    Input,
    Output,
    Body,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Parameter<Capture = crate::context::Identity> {
    Value(Slot<Value<Capture, Capture>>),
    Particle(Slot<Particle<Capture, Capture>>),
    Input(Slot<Input<Capture, Capture>>),
    Output(Slot<Output<Capture, Capture>>),
    Body(Slot<Body<Capture, Capture>>),
}

impl<Capture: Clone + Ord> Parameter<Capture> {
    pub(crate) fn identity(&self) -> (scope::Identity, usize, Sort) {
        match self {
            Self::Value(slot) => (slot.scope, slot.position, Sort::Value),
            Self::Particle(slot) => (slot.scope, slot.position, Sort::Particle),
            Self::Input(slot) => (slot.scope, slot.position, Sort::Input),
            Self::Output(slot) => (slot.scope, slot.position, Sort::Output),
            Self::Body(slot) => (slot.scope, slot.position, Sort::Body),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Declaration<Capture = crate::context::Identity> {
    pub(crate) scope: scope::Identity,
    pub(crate) parameter: Vec<Parameter<Capture>>,
    entry: BTreeMap<usize, Sort>,
}

impl<Capture: Clone + Ord> Declaration<Capture> {
    pub fn new(
        scope: scope::Identity,
        parameter: Vec<Parameter<Capture>>,
    ) -> Result<Self, Failure> {
        let mut entry = BTreeMap::new();
        for value in &parameter {
            let (owner, position, sort) = value.identity();
            if owner != scope {
                return Err(Failure::Scope(owner));
            }
            if entry.insert(position, sort).is_some() {
                return Err(Failure::Occupied { scope, position });
            }
        }
        Ok(Self {
            scope,
            parameter,
            entry,
        })
    }

    pub(crate) fn contains<Item>(&self, slot: &Slot<Item>, sort: Sort) -> Result<bool, Failure> {
        if slot.scope != self.scope {
            return Ok(false);
        }
        match self.entry.get(&slot.position) {
            Some(&expected) if expected == sort => Ok(true),
            Some(&expected) => Err(Failure::Sort {
                expected,
                actual: sort,
            }),
            None => Err(Failure::Binding {
                scope: slot.scope,
                position: slot.position,
            }),
        }
    }
}
