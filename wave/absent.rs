use crate::failure::Failure;
use photonic::laser::net::{Cycle, Exploration, Net};
use photonic::runtime::Limit;

pub enum Engine {}

impl Engine {
    // Metal requires macOS, so there is never a GPU to explore on.
    pub fn new() -> Result<Option<Self>, Failure> {
        Ok(None)
    }

    pub fn name(&self) -> &str {
        match *self {}
    }

    pub fn capacity(&self) -> usize {
        match *self {}
    }

    pub fn explore(
        &self,
        _: &mut Net,
        _: usize,
        _: Limit,
        _: Cycle,
    ) -> Result<Exploration, Failure> {
        match *self {}
    }
}
