use model::application::{self, Code, Request, Selection};
use model::configuration::Configuration;
use model::construction::Construction;
use model::context::{self, Frame};
use model::failure::Failure;
use model::flow::{Flow, Place};
use model::history::{Branch, History};
use model::introduction;
use model::occurrence::{Identity, Occurrence};
use model::structure::Value;
use model::world::{self, World};
use std::collections::BTreeSet;

fn state(value: Vec<Occurrence>, history: History) -> Configuration {
    Configuration::new(
        context::Identity(0),
        vec![World {
            identity: world::Identity(0),
            context: context::Identity(0),
            occurrence: value,
        }],
        vec![Frame {
            identity: context::Identity(0),
            parent: None,
            lexical: None,
            declaration: vec![],
            held: vec![],
        }],
        history,
    )
    .unwrap()
}

fn place(world: u64, occurrence: u64) -> Place {
    Place::World(world::Identity(world), Identity(occurrence))
}

#[test]
fn execution() {
    let role = super::source(1, Value::Atom("A".into()));
    let construction = Construction::new(context::Identity(0), History::default());
    let generated = construction
        .rule(
            construction
                .input([construction
                    .particle([construction.inspect(&role).unwrap()])
                    .unwrap()])
                .unwrap(),
            construction
                .output([construction
                    .destination(
                        construction.particle([construction.literal("B")]).unwrap(),
                        None,
                    )
                    .unwrap()])
                .unwrap(),
        )
        .unwrap();
    let initial = state(
        vec![super::source(0, Value::Atom("Seed".into())), role],
        History::default(),
    );
    let introduced =
        introduction::apply(&initial, world::Identity(0), &[Identity(0)], &generated).unwrap();
    introduced
        .flow
        .validate(&initial, &introduced.target)
        .unwrap();
    assert_eq!(introduced.read, BTreeSet::from([place(0, 1)]));
    assert_eq!(introduced.consumed, BTreeSet::from([place(0, 0)]));
    assert_eq!(
        introduced.flow.resource[&place(1, 2)],
        BTreeSet::from([place(0, 0)])
    );
    let result = application::apply(
        &introduced.target,
        &Request {
            code: Code::Local {
                world: world::Identity(1),
                occurrence: introduced.occurrence,
            },
            selection: vec![Selection {
                world: world::Identity(1),
                occurrence: vec![Identity(1)],
            }],
        },
    )
    .unwrap();
    result
        .flow
        .validate(&introduced.target, &result.target)
        .unwrap();
    let flow = introduced.flow.compose(&result.flow).unwrap();
    flow.validate(&initial, &result.target).unwrap();
    assert_eq!(flow.resource[&place(2, 3)], BTreeSet::from([place(0, 1)]));
    assert_eq!(
        introduced.flow.project(&result.read).unwrap(),
        BTreeSet::from([place(0, 0)])
    );
    assert_eq!(Flow::identity(&initial).compose(&flow).unwrap(), flow);
    assert_eq!(flow.compose(&Flow::identity(&result.target)).unwrap(), flow);
    assert!(
        result
            .target
            .world()
            .next()
            .unwrap()
            .occurrence
            .iter()
            .any(|value| value.value == Value::Atom("B".into()))
    );
    assert_eq!(
        introduction::apply(&result.target, world::Identity(2), &[], &generated),
        Err(Failure::Occurrence(Identity(1)))
    );
    assert_eq!(introduced.read, BTreeSet::from([place(0, 1)]));
}

#[test]
fn overlap() {
    let occurrence = super::source(0, Value::Atom("A".into()));
    let construction = Construction::new(context::Identity(0), History::default());
    let value = construction.inspect(&occurrence).unwrap();
    let state = state(vec![occurrence], History::default());
    let event = introduction::apply(&state, world::Identity(0), &[Identity(0)], &value).unwrap();
    assert_eq!(event.read, event.consumed);
    assert_eq!(event.target.world().next().unwrap().occurrence.len(), 1);
    assert_ne!(event.occurrence, Identity(0));
    assert_eq!(
        event.flow.resource[&place(1, 1)],
        BTreeSet::from([place(0, 0)])
    );
    assert_eq!(
        state.world().next().unwrap().occurrence[0].identity,
        Identity(0)
    );
}

#[test]
fn literal() {
    let construction = Construction::new(context::Identity(0), History::default());
    let state = state(vec![], History::default());
    let event =
        introduction::apply(&state, world::Identity(0), &[], &construction.literal("A")).unwrap();
    assert!(event.read.is_empty());
    assert!(event.consumed.is_empty());
    assert!(event.flow.resource[&place(1, 0)].is_empty());
    assert_eq!(
        event.flow.context[&world::Identity(1)],
        BTreeSet::from([world::Identity(0)])
    );
    event.flow.validate(&state, &event.target).unwrap();
}

#[test]
fn witness() {
    let construction = Construction::new(context::Identity(0), History::default());
    let occurrence = super::source(0, Value::Atom("A".into()));
    let value = construction.inspect(&occurrence).unwrap();
    let state = state(
        vec![super::source(0, Value::Atom("B".into()))],
        History::default(),
    );
    assert_eq!(
        introduction::apply(&state, world::Identity(0), &[], &value),
        Err(Failure::Identity(Identity(0)))
    );
    let conflicting = construction
        .inspect(&state.world().next().unwrap().occurrence[0])
        .unwrap();
    assert_eq!(
        construction.particle([value.clone(), conflicting.clone()]),
        Err(Failure::Identity(Identity(0)))
    );
    let repeated = construction
        .particle([value.clone(), value.clone()])
        .unwrap();
    assert_eq!(repeated.value().value().len(), 2);
    assert_eq!(repeated.evidence().witness().count(), 1);
    assert_eq!(repeated.evidence().witness().next(), Some(&occurrence));
    let mut scope = model::scope::Scope::new(model::scope::Identity(0));
    let left = scope.declare().unwrap();
    let right = scope.declare().unwrap();
    let mut environment = model::environment::Environment::new(scope.identity());
    environment.value = environment
        .value
        .bind(&left, value)
        .unwrap()
        .bind(&right, conflicting)
        .unwrap();
    let template = model::template::Value::Rule(Box::new(model::template::Rule {
        input: model::template::Input::Build(vec![]),
        output: model::template::Output::Build(vec![model::template::Destination {
            particle: model::template::Particle::Build(vec![
                model::template::Value::Reference(left),
                model::template::Value::Reference(right),
            ]),
            body: None,
        }]),
    }));
    assert_eq!(
        template.instantiate(&construction, &environment),
        Err(Failure::Identity(Identity(0)))
    );
    super::machine::compare(&template, &construction, &environment);
    let definition = model::generator::Definition {
        declaration: model::parameter::Declaration::new(model::scope::Identity(1), vec![]).unwrap(),
        term: model::generator::Term::Value(template),
    };
    assert_eq!(
        model::generator::Generator::close(definition, environment, &construction),
        Err(Failure::Identity(Identity(0)))
    );
}

#[test]
fn admission() {
    let fork = model::history::Identity(1);
    let history = History::default().decide(fork, Branch(0)).unwrap();
    let construction = Construction::new(context::Identity(0), history.clone());
    let value = construction.literal("A");
    let initial = state(vec![], History::default());
    assert!(matches!(
        introduction::apply(&initial, world::Identity(0), &[], &value),
        Err(Failure::History { .. })
    ));
    let current = state(vec![super::source(0, Value::Atom("Seed".into()))], history);
    assert_eq!(
        introduction::apply(
            &current,
            world::Identity(0),
            &[Identity(0), Identity(0)],
            &value
        ),
        Err(Failure::Repeated(Identity(0)))
    );
    assert_eq!(
        introduction::apply(&current, world::Identity(0), &[Identity(9)], &value),
        Err(Failure::Occurrence(Identity(9)))
    );
    assert!(introduction::apply(&current, world::Identity(0), &[], &value).is_ok());
    let foreign = Construction::new(context::Identity(7), current.history().clone()).literal("A");
    assert_eq!(
        introduction::apply(&current, world::Identity(0), &[], &foreign),
        Err(Failure::Context(context::Identity(7)))
    );
}

#[test]
fn capacity() {
    let construction = Construction::new(context::Identity(0), History::default());
    let state = state(
        vec![super::source(u64::MAX, Value::Atom("A".into()))],
        History::default(),
    );
    assert_eq!(
        introduction::apply(&state, world::Identity(0), &[], &construction.literal("B")),
        Err(Failure::Capacity)
    );
    assert_eq!(state.world().next().unwrap().occurrence.len(), 1);
}
