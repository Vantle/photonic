use crate::failure::Failure;
use network::input::{Input, Output, Sample};
use network::loss::{Loss, Weight};
use network::model::Model;
use network::optimizer::Optimizer;

pub enum Engine {}

impl Engine {
    pub fn new(_: &Model) -> Result<Self, Failure> {
        Err(Failure::Unavailable(
            "Metal requires macOS on Apple hardware".to_owned(),
        ))
    }

    pub fn name(&self) -> &str {
        match *self {}
    }

    pub fn load(&mut self, _: &[f32]) -> Result<(), Failure> {
        match *self {}
    }

    pub fn parameter(&mut self) -> Vec<f32> {
        match *self {}
    }

    pub fn prepare(&mut self, _: &Optimizer) -> Result<(), Failure> {
        match *self {}
    }

    pub fn optimizer(&mut self) -> Option<Optimizer> {
        match *self {}
    }

    pub fn infer(&mut self, _: &[&Input]) -> Result<Vec<Output>, Failure> {
        match *self {}
    }

    pub fn train(&mut self, _: &[&Sample], _: Weight, _: f32) -> Result<(Loss, f32), Failure> {
        match *self {}
    }
}
