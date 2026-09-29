use crate::failure::Failure;
use photonic::laser::net::{Cycle, Exploration, Net};
use photonic::runtime::Limit;

pub enum Engine {}

impl Engine {
    pub fn new() -> Result<Self, Failure> {
        Err(Failure::Metal(metal::failure::Failure::Unavailable(
            "Metal requires macOS on Apple hardware".to_owned(),
        )))
    }

    pub fn name(&self) -> &str {
        match *self {}
    }

    pub fn explore(&self, _: &mut Net, _: Limit, _: Cycle) -> Result<Exploration, Failure> {
        match *self {}
    }
}
