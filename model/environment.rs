use crate::binding::Binding;
use crate::failure::Failure;
use crate::scope;
use crate::structure::{Body, Input, Output, Particle, Value};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Environment {
    pub value: Binding<Value>,
    pub particle: Binding<Particle>,
    pub input: Binding<Input>,
    pub output: Binding<Output>,
    pub body: Binding<Body>,
}

impl Environment {
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
