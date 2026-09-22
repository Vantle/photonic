use crate::construction::Construction;
use crate::context::Reference;
use crate::environment::Environment;
use crate::failure::Failure;
use crate::fragment::Fragment;
use crate::slot::Slot;
use crate::structure;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value<Capture = crate::context::Identity> {
    Atom(String),
    Reference(Slot<structure::Value<Capture, Capture>>),
    Rule(Box<Rule<Capture>>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Particle<Capture = crate::context::Identity> {
    Reference(Slot<structure::Particle<Capture, Capture>>),
    Build(Vec<Value<Capture>>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Input<Capture = crate::context::Identity> {
    Reference(Slot<structure::Input<Capture, Capture>>),
    Build(Vec<Particle<Capture>>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Destination<Capture = crate::context::Identity> {
    pub particle: Particle<Capture>,
    pub body: Option<Body<Capture>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Output<Capture = crate::context::Identity> {
    Reference(Slot<structure::Output<Capture, Capture>>),
    Build(Vec<Destination<Capture>>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Body<Capture = crate::context::Identity> {
    Reference(Slot<structure::Body<Capture, Capture>>),
    Build(Vec<Rule<Capture>>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule<Capture = crate::context::Identity> {
    pub input: Input<Capture>,
    pub output: Output<Capture>,
}

impl<Capture: Clone + Ord> Value<Capture> {
    pub fn instantiate(
        &self,
        construction: &Construction<Capture, Capture>,
        environment: &Environment<Capture>,
    ) -> Result<Fragment<structure::Value<Capture, Capture>>, Failure> {
        construction.seal(self.defer(&construction.defer(), environment)?)
    }

    pub(crate) fn defer(
        &self,
        construction: &Construction<Reference<Capture>, Capture>,
        environment: &Environment<Capture>,
    ) -> Result<Fragment<structure::Value<Reference<Capture>, Capture>>, Failure> {
        match self {
            Self::Atom(atom) => Ok(construction.literal(atom.clone())),
            Self::Reference(slot) => construction.capture(environment.value.resolve(slot)?),
            Self::Rule(rule) => rule.defer(construction, environment),
        }
    }
}

impl<Capture: Clone + Ord> Particle<Capture> {
    pub fn instantiate(
        &self,
        construction: &Construction<Capture, Capture>,
        environment: &Environment<Capture>,
    ) -> Result<Fragment<structure::Particle<Capture, Capture>>, Failure> {
        construction.seal(self.defer(&construction.defer(), environment)?)
    }

    pub(crate) fn defer(
        &self,
        construction: &Construction<Reference<Capture>, Capture>,
        environment: &Environment<Capture>,
    ) -> Result<Fragment<structure::Particle<Reference<Capture>, Capture>>, Failure> {
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

impl<Capture: Clone + Ord> Input<Capture> {
    pub fn instantiate(
        &self,
        construction: &Construction<Capture, Capture>,
        environment: &Environment<Capture>,
    ) -> Result<Fragment<structure::Input<Capture, Capture>>, Failure> {
        construction.seal(self.defer(&construction.defer(), environment)?)
    }

    pub(crate) fn defer(
        &self,
        construction: &Construction<Reference<Capture>, Capture>,
        environment: &Environment<Capture>,
    ) -> Result<Fragment<structure::Input<Reference<Capture>, Capture>>, Failure> {
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

impl<Capture: Clone + Ord> Destination<Capture> {
    pub fn instantiate(
        &self,
        construction: &Construction<Capture, Capture>,
        environment: &Environment<Capture>,
    ) -> Result<Fragment<structure::Destination<Capture, Capture>>, Failure> {
        construction.seal(self.defer(&construction.defer(), environment)?)
    }

    pub(crate) fn defer(
        &self,
        construction: &Construction<Reference<Capture>, Capture>,
        environment: &Environment<Capture>,
    ) -> Result<Fragment<structure::Destination<Reference<Capture>, Capture>>, Failure> {
        construction.destination(
            self.particle.defer(construction, environment)?,
            self.body
                .as_ref()
                .map(|body| body.defer(construction, environment))
                .transpose()?,
        )
    }
}

impl<Capture: Clone + Ord> Output<Capture> {
    pub fn instantiate(
        &self,
        construction: &Construction<Capture, Capture>,
        environment: &Environment<Capture>,
    ) -> Result<Fragment<structure::Output<Capture, Capture>>, Failure> {
        construction.seal(self.defer(&construction.defer(), environment)?)
    }

    pub(crate) fn defer(
        &self,
        construction: &Construction<Reference<Capture>, Capture>,
        environment: &Environment<Capture>,
    ) -> Result<Fragment<structure::Output<Reference<Capture>, Capture>>, Failure> {
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

impl<Capture: Clone + Ord> Body<Capture> {
    pub fn instantiate(
        &self,
        construction: &Construction<Capture, Capture>,
        environment: &Environment<Capture>,
    ) -> Result<Fragment<structure::Body<Capture, Capture>>, Failure> {
        construction.seal(self.defer(&construction.defer(), environment)?)
    }

    pub(crate) fn defer(
        &self,
        construction: &Construction<Reference<Capture>, Capture>,
        environment: &Environment<Capture>,
    ) -> Result<Fragment<structure::Body<Reference<Capture>, Capture>>, Failure> {
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

impl<Capture: Clone + Ord> Rule<Capture> {
    pub fn instantiate(
        &self,
        construction: &Construction<Capture, Capture>,
        environment: &Environment<Capture>,
    ) -> Result<Fragment<structure::Value<Capture, Capture>>, Failure> {
        construction.seal(self.defer(&construction.defer(), environment)?)
    }

    pub(crate) fn defer(
        &self,
        construction: &Construction<Reference<Capture>, Capture>,
        environment: &Environment<Capture>,
    ) -> Result<Fragment<structure::Value<Reference<Capture>, Capture>>, Failure> {
        construction.rule(
            self.input.defer(construction, environment)?,
            self.output.defer(construction, environment)?,
        )
    }
}
