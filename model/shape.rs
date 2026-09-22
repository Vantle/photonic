use crate::context::{Identity, Reference};
use crate::structure;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Shape {
    Atom(String),
    Rule {
        context: Reference,
        input: Vec<Vec<Self>>,
        output: Vec<Destination>,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Destination {
    particle: Vec<Shape>,
    body: Option<Body>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Body {
    context: Reference,
    rule: Vec<Shape>,
}

trait Context: Clone + Ord {
    fn resolve(&self, frame: &BTreeMap<Identity, Identity>) -> Reference;
}

impl Context for Identity {
    fn resolve(&self, frame: &BTreeMap<Identity, Identity>) -> Reference {
        Reference::Captured(frame[self])
    }
}

impl Context for Reference {
    fn resolve(&self, frame: &BTreeMap<Identity, Identity>) -> Reference {
        match self {
            Self::Captured(identity) => identity.resolve(frame),
            Self::Local(depth) => Self::Local(*depth),
        }
    }
}

fn value<Owner: Context>(
    source: &structure::Value<Owner>,
    frame: &BTreeMap<Identity, Identity>,
) -> Shape {
    match source {
        structure::Value::Atom(atom) => Shape::Atom(atom.clone()),
        structure::Value::Rule(source) => rule(source, frame),
    }
}

fn particle<Owner: Context>(
    source: &structure::Particle<Owner>,
    frame: &BTreeMap<Identity, Identity>,
) -> Vec<Shape> {
    let mut result = source
        .value()
        .iter()
        .map(|source| value(source, frame))
        .collect::<Vec<_>>();
    result.sort();
    result
}

fn rule<Owner: Context>(
    source: &structure::Rule<Owner>,
    frame: &BTreeMap<Identity, Identity>,
) -> Shape {
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

impl Shape {
    pub(crate) fn value(source: &structure::Value, frame: &BTreeMap<Identity, Identity>) -> Self {
        value(source, frame)
    }

    pub(crate) fn declaration(
        source: &[structure::Rule],
        frame: &BTreeMap<Identity, Identity>,
    ) -> Vec<Self> {
        let mut result = source
            .iter()
            .map(|source| rule(source, frame))
            .collect::<Vec<_>>();
        result.sort();
        result
    }
}
