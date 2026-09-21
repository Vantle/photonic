use crate::scope;
use std::marker::PhantomData;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Slot<Value> {
    pub(crate) scope: scope::Identity,
    pub(crate) position: usize,
    marker: PhantomData<fn() -> Value>,
}

impl<Value> Slot<Value> {
    pub(crate) fn new(scope: scope::Identity, position: usize) -> Self {
        Self {
            scope,
            position,
            marker: PhantomData,
        }
    }
}
