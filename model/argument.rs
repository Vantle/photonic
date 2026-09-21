use crate::fragment::Fragment;
use crate::structure::{Body, Input, Output, Particle, Value};

pub enum Argument {
    Value(Fragment<Value>),
    Particle(Fragment<Particle>),
    Input(Fragment<Input>),
    Output(Fragment<Output>),
    Body(Fragment<Body>),
}

impl Argument {
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
