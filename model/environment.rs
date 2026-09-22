use crate::binding::Binding;
use crate::failure::Failure;
use crate::scope;
use crate::structure::{Body, Input, Output, Particle, Value};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Environment<Capture = crate::context::Identity> {
    pub value: Binding<Value<Capture, Capture>>,
    pub particle: Binding<Particle<Capture, Capture>>,
    pub input: Binding<Input<Capture, Capture>>,
    pub output: Binding<Output<Capture, Capture>>,
    pub body: Binding<Body<Capture, Capture>>,
}

impl<Capture: Clone + Ord> Environment<Capture> {
    pub fn new(scope: scope::Identity) -> Self {
        Self {
            value: Binding::new(scope),
            particle: Binding::new(scope),
            input: Binding::new(scope),
            output: Binding::new(scope),
            body: Binding::new(scope),
        }
    }

    pub fn nested(&self, scope: scope::Identity) -> Result<Self, Failure> {
        Ok(Self {
            value: self.value.nested(scope)?,
            particle: self.particle.nested(scope)?,
            input: self.input.nested(scope)?,
            output: self.output.nested(scope)?,
            body: self.body.nested(scope)?,
        })
    }
}
