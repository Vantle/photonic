use crate::context::{Identity, Reference};
use crate::structure;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Shape<Capture = Identity> {
    Atom(String),
    Rule {
        context: Reference<Capture>,
        input: Vec<Vec<Self>>,
        output: Vec<Destination<Capture>>,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Destination<Capture> {
    particle: Vec<Shape<Capture>>,
    body: Option<Body<Capture>>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Body<Capture> {
    context: Reference<Capture>,
    rule: Vec<Shape<Capture>>,
}

trait Context<Capture>: Clone + Ord {
    fn resolve(&self, frame: &BTreeMap<Capture, Capture>) -> Reference<Capture>;
}

impl<Capture: Clone + Ord> Context<Capture> for Capture {
    fn resolve(&self, frame: &BTreeMap<Capture, Capture>) -> Reference<Capture> {
        Reference::Captured(frame[self].clone())
    }
}

impl<Capture: Clone + Ord> Context<Capture> for Reference<Capture> {
    fn resolve(&self, frame: &BTreeMap<Capture, Capture>) -> Self {
        match self {
            Self::Captured(identity) => identity.resolve(frame),
            Self::Local(depth) => Self::Local(*depth),
        }
    }
}

fn value<Capture: Clone + Ord, Owner: Context<Capture>>(
    source: &structure::Value<Owner, Capture>,
    frame: &BTreeMap<Capture, Capture>,
) -> Shape<Capture> {
    match source {
        structure::Value::Atom(atom) => Shape::Atom(atom.clone()),
        structure::Value::Rule(source) => rule(source, frame),
    }
}

fn particle<Capture: Clone + Ord, Owner: Context<Capture>>(
    source: &structure::Particle<Owner, Capture>,
    frame: &BTreeMap<Capture, Capture>,
) -> Vec<Shape<Capture>> {
    let mut result = source
        .value()
        .iter()
        .map(|source| value(source, frame))
        .collect::<Vec<_>>();
    result.sort();
    result
}

fn rule<Capture: Clone + Ord, Owner: Context<Capture>>(
    source: &structure::Rule<Owner, Capture>,
    frame: &BTreeMap<Capture, Capture>,
) -> Shape<Capture> {
    let mut input = source
        .input
        .particle()
        .iter()
        .map(|source| particle(source, frame))
        .collect::<Vec<_>>();
    input.sort();
    let mut output = source
        .output
        .destination()
        .iter()
        .map(|source| {
            let body = source.body.as_ref().map(|source| {
                let mut declaration = source
                    .rule()
                    .iter()
                    .map(|source| rule(source, frame))
                    .collect::<Vec<_>>();
                declaration.sort();
                Body {
                    context: source.context().resolve(frame),
                    rule: declaration,
                }
            });
            Destination {
                particle: particle(&source.particle, frame),
                body,
            }
        })
        .collect::<Vec<_>>();
    output.sort();
    Shape::Rule {
        context: source.context.resolve(frame),
        input,
        output,
    }
}

impl<Capture: Clone + Ord> Shape<Capture> {
    pub(crate) fn value(
        source: &structure::Value<Capture, Capture>,
        frame: &BTreeMap<Capture, Capture>,
    ) -> Self {
        value(source, frame)
    }

    pub(crate) fn declaration(
        source: &[structure::Rule<Capture, Capture>],
        frame: &BTreeMap<Capture, Capture>,
    ) -> Vec<Self> {
        let mut result = source
            .iter()
            .map(|source| rule(source, frame))
            .collect::<Vec<_>>();
        result.sort();
        result
    }
}
