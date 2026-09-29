use crate::failure::Failure;
use crate::shape::Shape;
use photonic::laser::ground::{Exploration, Ground};
use photonic::runtime::Limit;

pub enum Engine {}

impl Engine {
    pub fn new() -> Result<Self, Failure> {
        Self::shaped(Shape::default())
    }

    pub fn shaped(_: Shape) -> Result<Self, Failure> {
        Err(Failure::Metal(metal::failure::Failure::Unavailable(
            "Metal requires macOS on Apple hardware".to_owned(),
        )))
    }

    pub fn name(&self) -> &str {
        match *self {}
    }

    pub fn explore(&self, _: &mut Ground, _: Limit, _: bool) -> Result<Exploration, Failure> {
        match *self {}
    }
}
