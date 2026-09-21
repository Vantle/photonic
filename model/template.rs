use crate::construction::Construction;
use crate::context::Reference;
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
        construction.seal(self.defer(&construction.defer(), environment)?)
    }

    pub(crate) fn defer(
        &self,
        construction: &Construction<Reference>,
        environment: &Environment,
    ) -> Result<Fragment<structure::Value<Reference>>, Failure> {
        match self {
            Self::Atom(atom) => Ok(construction.literal(atom.clone())),
            Self::Reference(slot) => construction.capture(environment.value.resolve(slot)?),
            Self::Rule(rule) => rule.defer(construction, environment),
        }
    }
}

impl Particle {
    pub fn instantiate(
        &self,
        construction: &Construction,
        environment: &Environment,
    ) -> Result<Fragment<structure::Particle>, Failure> {
        construction.seal(self.defer(&construction.defer(), environment)?)
    }

    pub(crate) fn defer(
        &self,
        construction: &Construction<Reference>,
        environment: &Environment,
    ) -> Result<Fragment<structure::Particle<Reference>>, Failure> {
        match self {
            Self::Reference(slot) => construction.capture(environment.particle.resolve(slot)?),
            Self::Build(value) => construction.particle(
                value
                    .iter()
                    .map(|value| value.defer(construction, environment))
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
        construction.seal(self.defer(&construction.defer(), environment)?)
    }

    pub(crate) fn defer(
        &self,
        construction: &Construction<Reference>,
        environment: &Environment,
    ) -> Result<Fragment<structure::Input<Reference>>, Failure> {
        match self {
            Self::Reference(slot) => construction.capture(environment.input.resolve(slot)?),
            Self::Build(particle) => construction.input(
                particle
                    .iter()
                    .map(|particle| particle.defer(construction, environment))
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
        construction.seal(self.defer(&construction.defer(), environment)?)
    }

    pub(crate) fn defer(
        &self,
        construction: &Construction<Reference>,
        environment: &Environment,
    ) -> Result<Fragment<structure::Destination<Reference>>, Failure> {
        construction.destination(
            self.particle.defer(construction, environment)?,
            self.body
                .as_ref()
                .map(|body| body.defer(construction, environment))
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
        construction.seal(self.defer(&construction.defer(), environment)?)
    }

    pub(crate) fn defer(
        &self,
        construction: &Construction<Reference>,
        environment: &Environment,
    ) -> Result<Fragment<structure::Output<Reference>>, Failure> {
        match self {
            Self::Reference(slot) => construction.capture(environment.output.resolve(slot)?),
            Self::Build(destination) => construction.output(
                destination
                    .iter()
                    .map(|destination| destination.defer(construction, environment))
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
        construction.seal(self.defer(&construction.defer(), environment)?)
    }

    pub(crate) fn defer(
        &self,
        construction: &Construction<Reference>,
        environment: &Environment,
    ) -> Result<Fragment<structure::Body<Reference>>, Failure> {
        match self {
            Self::Reference(slot) => construction.capture(environment.body.resolve(slot)?),
            Self::Build(rule) => {
                let local = construction.local();
                construction.body(
                    rule.iter()
                        .map(|rule| local.definition(rule.defer(&local, environment)?))
                        .collect::<Result<Vec<_>, _>>()?,
                )
            }
        }
    }
}

impl Rule {
    pub fn instantiate(
        &self,
        construction: &Construction,
        environment: &Environment,
    ) -> Result<Fragment<structure::Value>, Failure> {
        construction.seal(self.defer(&construction.defer(), environment)?)
    }

    pub(crate) fn defer(
        &self,
        construction: &Construction<Reference>,
        environment: &Environment,
    ) -> Result<Fragment<structure::Value<Reference>>, Failure> {
        construction.rule(
            self.input.defer(construction, environment)?,
            self.output.defer(construction, environment)?,
        )
    }
}
