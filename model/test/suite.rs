mod generator;
mod machine;
mod projection;
mod template;

use model::construction::Construction;
use model::context;
use model::failure::Failure;
use model::fragment::Fragment;
use model::history::{Branch, History};
use model::occurrence::{Identity, Occurrence};
use model::structure::{Body, Destination, Input, Output, Particle, Rule, Value};
use std::collections::BTreeSet;

fn field(construction: &Construction, role: &str, value: Fragment<Value>) -> Fragment<Value> {
    let input = construction
        .input([construction.particle([construction.literal(role)]).unwrap()])
        .unwrap();
    let output = construction
        .output([construction
            .destination(construction.particle([value]).unwrap(), None)
            .unwrap()])
        .unwrap();
    construction.rule(input, output).unwrap()
}

fn rule(context: u64, input: &str, output: Value) -> Value {
    Value::Rule(Box::new(Rule {
        input: Input::new(vec![Particle::new(vec![Value::Atom(input.into())])]),
        output: Output::new(vec![Destination {
            particle: Particle::new(vec![output]),
            body: None,
        }]),
        context: context::Identity(context),
    }))
}

fn source(identity: u64, value: Value) -> Occurrence {
    Occurrence {
        identity: Identity(identity),
        value,
        history: History::default(),
    }
}

#[test]
fn unknown() {
    let construction = Construction::new(context::Identity(0), History::default());
    for role in ["Unseen", "Role", "λ", "3"] {
        for payload in ["Unseen", "Payload", "λ", "3"] {
            let value = field(&construction, role, construction.literal(payload));
            assert_eq!(value.value(), &rule(0, role, Value::Atom(payload.into())));
            assert!(value.evidence().read().is_empty());
        }
    }
}

#[test]
fn supplied() {
    let construction = Construction::new(context::Identity(0), History::default());
    let role = source(1, Value::Atom("Unseen".into()));
    let payload = source(2, rule(7, "Call", Value::Atom("Payload".into())));
    let input = construction
        .input([construction
            .particle([construction.inspect(&role).unwrap()])
            .unwrap()])
        .unwrap();
    let output = construction
        .output([construction
            .destination(
                construction
                    .particle([construction.inspect(&payload).unwrap()])
                    .unwrap(),
                None,
            )
            .unwrap()])
        .unwrap();
    let result = construction.rule(input, output).unwrap();
    assert_eq!(result.value(), &rule(0, "Unseen", payload.value));
    assert_eq!(
        &result.evidence().read(),
        &BTreeSet::from([Identity(1), Identity(2)])
    );
    assert_eq!(
        result.evidence().context(),
        &BTreeSet::from([context::Identity(0), context::Identity(7)])
    );
}

#[test]
fn reconstruction() {
    let construction = Construction::new(context::Identity(91), History::default());
    let mut value = Value::Atom("Payload".into());
    for depth in 0..16 {
        value = rule(depth, "Step", value);
        let occurrence = source(depth, value.clone());
        let fragment = construction.inspect(&occurrence).unwrap();
        let definition = construction.definition(fragment).unwrap();
        let inspection = construction.open(definition).unwrap();
        let result = construction.close(inspection).unwrap();
        assert_eq!(result.value(), &value);
        assert_eq!(
            &result.evidence().read(),
            &BTreeSet::from([Identity(depth)])
        );
        assert_eq!(
            result.evidence().context(),
            &(0..=depth)
                .chain([91])
                .map(context::Identity)
                .collect::<BTreeSet<_>>()
        );
    }
}

#[test]
fn boundary() {
    let construction = Construction::new(context::Identity(0), History::default());
    let empty = construction.input([]).unwrap();
    let single = construction
        .input([construction.particle([]).unwrap()])
        .unwrap();
    let double = construction
        .input([
            construction.particle([]).unwrap(),
            construction.particle([]).unwrap(),
        ])
        .unwrap();
    assert_ne!(empty, single);
    assert_ne!(single, double);
    assert_eq!(empty.value().particle().len(), 0);
    assert_eq!(single.value().particle().len(), 1);
    assert_eq!(double.value().particle().len(), 2);
    let absent = construction
        .destination(construction.particle([]).unwrap(), None)
        .unwrap();
    let present = construction
        .destination(
            construction.particle([]).unwrap(),
            Some(construction.body([]).unwrap()),
        )
        .unwrap();
    assert_ne!(absent, present);
    assert_ne!(
        construction.output([]).unwrap(),
        construction.output([absent]).unwrap()
    );
    assert_eq!(
        construction.definition(construction.literal("Atom")),
        Err(Failure::Rule)
    );
}

#[test]
fn multiplicity() {
    let construction = Construction::new(context::Identity(0), History::default());
    let left = construction
        .particle([construction.literal("A"), construction.literal("B")])
        .unwrap();
    let right = construction
        .particle([construction.literal("B"), construction.literal("A")])
        .unwrap();
    assert_eq!(left, right);
    let repeated = construction
        .particle([construction.literal("A"), construction.literal("A")])
        .unwrap();
    assert_eq!(repeated.value().value().len(), 2);
    let separate = construction.input([left.clone(), right]).unwrap();
    let combined = construction
        .input([construction
            .particle([
                construction.literal("A"),
                construction.literal("B"),
                construction.literal("A"),
                construction.literal("B"),
            ])
            .unwrap()])
        .unwrap();
    assert_ne!(separate, combined);
    assert_eq!(separate.value().particle().len(), 2);
    assert_eq!(construction.input([left.clone(), left]).unwrap(), separate);
}

#[test]
fn capture() {
    let construction = Construction::new(context::Identity(99), History::default());
    let left = source(1, rule(1, "A", Value::Atom("Result".into())));
    let right = source(2, rule(2, "A", Value::Atom("Result".into())));
    assert_ne!(left.value, right.value);
    let payload = construction
        .particle([
            construction.inspect(&left).unwrap(),
            construction.inspect(&right).unwrap(),
        ])
        .unwrap();
    let wrapper = construction
        .rule(
            construction.input([]).unwrap(),
            construction
                .output([construction.destination(payload, None).unwrap()])
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        wrapper.evidence().context(),
        &BTreeSet::from([
            context::Identity(1),
            context::Identity(2),
            context::Identity(99),
        ])
    );
    assert_eq!(
        &wrapper.evidence().read(),
        &BTreeSet::from([Identity(1), Identity(2)])
    );
    let Value::Rule(definition) = wrapper.value() else {
        panic!("constructed wrapper must be a rule");
    };
    assert_eq!(definition.context, context::Identity(99));
    assert_eq!(
        definition.output.destination()[0].particle,
        Particle::new(vec![left.value.clone(), right.value])
    );
    let fragment = construction.inspect(&left).unwrap();
    let copied = construction.particle([fragment.clone(), fragment]).unwrap();
    assert_eq!(copied.value().value().len(), 2);
    assert_eq!(&copied.evidence().read(), &BTreeSet::from([Identity(1)]));
}

#[test]
fn body() {
    let declaration = Rule {
        input: Input::default(),
        output: Output::default(),
        context: context::Identity(7),
    };
    let value = Value::Rule(Box::new(Rule {
        input: Input::default(),
        output: Output::new(vec![Destination {
            particle: Particle::default(),
            body: Some(Body::new(context::Identity(5), vec![declaration])),
        }]),
        context: context::Identity(3),
    }));
    let construction = Construction::new(context::Identity(11), History::default());
    let fragment = construction.inspect(&source(1, value.clone())).unwrap();
    let definition = construction.definition(fragment).unwrap();
    let inspection = construction.open(definition).unwrap();
    let result = construction.close(inspection).unwrap();
    assert_eq!(result.value(), &value);
    assert_eq!(
        result.evidence().context(),
        &[3, 5, 7, 11]
            .into_iter()
            .map(context::Identity)
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn replacement() {
    let construction = Construction::new(context::Identity(2), History::default());
    let occurrence = source(4, rule(1, "A", Value::Atom("Old".into())));
    let original = occurrence.clone();
    let value = construction.inspect(&occurrence).unwrap();
    let mut inspection = construction
        .open(construction.definition(value).unwrap())
        .unwrap();
    inspection.output = construction
        .output([construction
            .destination(
                construction
                    .particle([construction.literal("New")])
                    .unwrap(),
                None,
            )
            .unwrap()])
        .unwrap();
    let result = construction.close(inspection).unwrap();
    assert_eq!(result.value(), &rule(1, "A", Value::Atom("New".into())));
    assert_eq!(&result.evidence().read(), &BTreeSet::from([Identity(4)]));
    assert_eq!(occurrence, original);
}

#[test]
fn origin() {
    let fork = model::history::Identity(1);
    let history = History::default().decide(fork, Branch(0)).unwrap();
    let construction = Construction::new(context::Identity(2), history.clone());
    let occurrence = Occurrence {
        identity: Identity(4),
        value: rule(1, "A", Value::Atom("Old".into())),
        history,
    };
    let root = Construction::new(context::Identity(9), History::default());
    for same in [false, true] {
        let value = construction.inspect(&occurrence).unwrap();
        let mut inspection = construction
            .open(construction.definition(value).unwrap())
            .unwrap();
        inspection.input = root.input([]).unwrap();
        inspection.output = root.output([]).unwrap();
        if same {
            let value = construction.close(inspection).unwrap();
            assert_eq!(&value.evidence().read(), &BTreeSet::from([Identity(4)]));
            assert!(value.evidence().context().contains(&context::Identity(1)));
        } else {
            assert_eq!(
                root.close(inspection),
                Err(Failure::History {
                    identity: fork,
                    expected: Branch(0),
                    actual: None,
                })
            );
        }
    }
}

#[test]
fn matrix() {
    let mut history = Vec::new();
    for encoding in 0..27 {
        let mut value = History::default();
        let mut choice = Vec::new();
        let mut cursor = encoding;
        for identity in 0..3 {
            let branch = match cursor % 3 {
                0 => None,
                value => Some(Branch(value - 1)),
            };
            if let Some(branch) = branch {
                value = value
                    .decide(model::history::Identity(identity), branch)
                    .unwrap();
            }
            choice.push(branch);
            cursor /= 3;
        }
        history.push((value, choice));
    }
    for (target, expected) in &history {
        for (source, actual) in &history {
            let compatible = actual
                .iter()
                .zip(expected)
                .all(|(source, target)| source.is_none() || source == target);
            assert_eq!(target.permits(source).is_ok(), compatible);
            let construction = Construction::new(context::Identity(0), target.clone());
            let occurrence = Occurrence {
                identity: Identity(0),
                value: Value::Atom("Payload".into()),
                history: source.clone(),
            };
            assert_eq!(construction.inspect(&occurrence).is_ok(), compatible);
        }
    }
}

#[test]
fn history() {
    let fork = model::history::Identity(1);
    let root = History::default();
    let left = root.decide(fork, Branch(0)).unwrap();
    let right = root.decide(fork, Branch(1)).unwrap();
    let source = Occurrence {
        identity: Identity(1),
        value: Value::Atom("Payload".into()),
        history: left.clone(),
    };
    let construction = Construction::new(context::Identity(0), left.clone());
    let fragment = construction.inspect(&source).unwrap();
    let competing = Construction::new(context::Identity(0), right);
    assert_eq!(
        competing.inspect(&source),
        Err(Failure::History {
            identity: fork,
            expected: Branch(0),
            actual: Some(Branch(1)),
        })
    );
    assert!(competing.particle([fragment.clone()]).is_err());
    let ancestor = Construction::new(context::Identity(0), root);
    assert_eq!(
        ancestor.inspect(&source),
        Err(Failure::History {
            identity: fork,
            expected: Branch(0),
            actual: None,
        })
    );
    let descendant = left.decide(model::history::Identity(2), Branch(3)).unwrap();
    let continued = Construction::new(context::Identity(0), descendant.clone());
    let retained = continued.particle([fragment]).unwrap();
    assert_eq!(retained.evidence().history(), &descendant);
    assert_eq!(left.decide(fork, Branch(0)).unwrap(), left);
    assert!(left.decide(fork, Branch(1)).is_err());
}

#[test]
fn generation() {
    let construction = Construction::new(context::Identity(0), History::default());
    for depth in [0, 1, 2, 3, 8, 16, 32] {
        let mut value = construction.literal("Seed");
        for _ in 0..depth {
            value = field(&construction, "Step", value);
        }
        let mut cursor = value.value();
        for _ in 0..depth {
            let Value::Rule(definition) = cursor else {
                panic!("every generated level must be structural code");
            };
            assert_eq!(definition.input.particle().len(), 1);
            assert_eq!(definition.output.destination().len(), 1);
            cursor = &definition.output.destination()[0].particle.value()[0];
        }
        assert_eq!(cursor, &Value::Atom("Seed".into()));
    }
}
mod activation;
mod application;
mod declaration;
mod flow;
mod introduction;
