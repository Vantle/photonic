use crate::construction::Construction;
use crate::environment::Environment;
use crate::failure::Failure;
use crate::fragment::Fragment;
use crate::slot::Slot;
use crate::structure;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Atom(String),
    Reference(Slot<structure::Value>),
    Rule(Box<Rule>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Particle {
    Reference(Slot<structure::Particle>),
    Build(Vec<Value>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Input {
    Reference(Slot<structure::Input>),
    Build(Vec<Particle>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Destination {
    pub particle: Particle,
    pub body: Option<Body>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Output {
    Reference(Slot<structure::Output>),
    Build(Vec<Destination>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Body {
    Reference(Slot<structure::Body>),
    Build(Vec<Rule>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule {
    pub input: Input,
    pub output: Output,
}

impl Value {
    pub fn instantiate(
        &self,
        construction: &Construction,
        environment: &Environment,
    ) -> Result<Fragment<structure::Value>, Failure> {
        match self {
            Self::Atom(atom) => Ok(construction.literal(atom.clone())),
            Self::Reference(slot) => construction.accept(environment.value.resolve(slot)?),
            Self::Rule(rule) => rule.instantiate(construction, environment),
        }
    }
}

impl Particle {
    pub fn instantiate(
        &self,
        construction: &Construction,
        environment: &Environment,
    ) -> Result<Fragment<structure::Particle>, Failure> {
        match self {
            Self::Reference(slot) => construction.accept(environment.particle.resolve(slot)?),
            Self::Build(value) => construction.particle(
                value
                    .iter()
                    .map(|value| value.instantiate(construction, environment))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        }
    }
}

impl Input {
    pub fn instantiate(
        &self,
        construction: &Construction,
        environment: &Environment,
    ) -> Result<Fragment<structure::Input>, Failure> {
        match self {
            Self::Reference(slot) => construction.accept(environment.input.resolve(slot)?),
            Self::Build(particle) => construction.input(
                particle
                    .iter()
                    .map(|particle| particle.instantiate(construction, environment))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        }
    }
}

impl Destination {
    pub fn instantiate(
        &self,
        construction: &Construction,
        environment: &Environment,
    ) -> Result<Fragment<structure::Destination>, Failure> {
        construction.destination(
            self.particle.instantiate(construction, environment)?,
            self.body
                .as_ref()
                .map(|body| body.instantiate(construction, environment))
                .transpose()?,
        )
    }
}

impl Output {
    pub fn instantiate(
        &self,
        construction: &Construction,
        environment: &Environment,
    ) -> Result<Fragment<structure::Output>, Failure> {
        match self {
            Self::Reference(slot) => construction.accept(environment.output.resolve(slot)?),
            Self::Build(destination) => construction.output(
                destination
                    .iter()
                    .map(|destination| destination.instantiate(construction, environment))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        }
    }
}

impl Body {
    pub fn instantiate(
        &self,
        construction: &Construction,
        environment: &Environment,
    ) -> Result<Fragment<structure::Body>, Failure> {
        match self {
            Self::Reference(slot) => construction.accept(environment.body.resolve(slot)?),
            Self::Build(rule) => construction.body(
                rule.iter()
                    .map(|rule| {
                        construction.definition(rule.instantiate(construction, environment)?)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        }
    }
}

impl Rule {
    pub fn instantiate(
        &self,
        construction: &Construction,
        environment: &Environment,
    ) -> Result<Fragment<structure::Value>, Failure> {
        construction.rule(
            self.input.instantiate(construction, environment)?,
            self.output.instantiate(construction, environment)?,
        )
    }
}
