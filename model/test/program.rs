use model::configuration::Configuration;
use model::context::{Frame, Identity, Reference};
use model::history::History;
use model::occurrence::{self, Occurrence};
use model::structure::{Body, Destination, Input, Output, Particle, Rule, Value};
use model::world::{self, World};
use photonic::source;

fn value(source: &source::Value) -> Value<Reference> {
    match source {
        source::Value::Atom(atom) => Value::Atom(atom.clone()),
        source::Value::Rule { rule: definition } => Value::Rule(Box::new(rule(definition))),
    }
}

fn particle(source: &[source::Value]) -> Particle<Reference> {
    Particle::new(source.iter().map(value).collect())
}

fn rule(source: &source::Definition) -> Rule<Reference> {
    Rule {
        context: Reference::Local(0),
        input: Input::new(source.input.iter().map(|source| particle(source)).collect()),
        output: Output::new(
            source
                .output
                .iter()
                .map(|source| Destination {
                    particle: particle(&source.particle),
                    body: source.body.as_ref().map(|source| {
                        Body::nested(Reference::Local(0), source.iter().map(rule).collect())
                    }),
                })
                .collect(),
        ),
    }
}

pub fn read(source: &str) -> Configuration {
    let source = photonic::lowering::parse(source).unwrap();
    let declaration = Body::bind(Identity(0), source.rule.iter().map(rule).collect())
        .unwrap()
        .activate(Identity(0))
        .unwrap();
    let initial = Rule {
        context: Reference::Local(0),
        input: Input::default(),
        output: Output::new(
            source
                .initial
                .iter()
                .map(|source| Destination {
                    particle: particle(source),
                    body: None,
                })
                .collect(),
        ),
    };
    let initial = Body::bind(Identity(0), vec![initial])
        .unwrap()
        .activate(Identity(0))
        .unwrap();
    let mut identity = 0;
    let world = initial[0]
        .output
        .destination()
        .iter()
        .enumerate()
        .map(|(position, source)| World {
            identity: world::Identity(position as u64),
            context: Identity(0),
            occurrence: source
                .particle
                .value()
                .iter()
                .map(|value| {
                    let occurrence = Occurrence {
                        identity: occurrence::Identity(identity),
                        value: value.clone(),
                        history: History::default(),
                    };
                    identity += 1;
                    occurrence
                })
                .collect(),
        })
        .collect();
    Configuration::new(
        Identity(0),
        world,
        vec![Frame {
            identity: Identity(0),
            parent: None,
            lexical: None,
            declaration,
            held: vec![],
        }],
        History::default(),
    )
    .unwrap()
}
