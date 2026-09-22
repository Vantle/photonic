use crate::fragment::Fragment;
use crate::structure::{Body, Input, Output, Particle, Value};

pub enum Argument<Capture = crate::context::Identity> {
    Value(Fragment<Value<Capture, Capture>>),
    Particle(Fragment<Particle<Capture, Capture>>),
    Input(Fragment<Input<Capture, Capture>>),
    Output(Fragment<Output<Capture, Capture>>),
    Body(Fragment<Body<Capture, Capture>>),
}

impl<Capture: Clone + Ord> Argument<Capture> {
    pub(crate) fn sort(&self) -> crate::parameter::Sort {
        match self {
            Self::Value(_) => crate::parameter::Sort::Value,
            Self::Particle(_) => crate::parameter::Sort::Particle,
            Self::Input(_) => crate::parameter::Sort::Input,
            Self::Output(_) => crate::parameter::Sort::Output,
            Self::Body(_) => crate::parameter::Sort::Body,
        }
    }
}
