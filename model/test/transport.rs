use model::admission::{self, Witness};
use model::application::{Code, Request, Selection};
use model::configuration::Configuration;
use model::construction::Construction;
use model::context;
use model::flow::Place;
use model::fragment::Fragment;
use model::history::History;
use model::occurrence::Identity;
use model::path::{Path, Step};
use model::projection;
use model::structure::{Body, Destination, Input, Output, Particle, Rule, Value};
use model::world;
use std::collections::BTreeSet;

fn remove(path: &Path, occurrence: Identity, label: &str) -> Path {
    path.advance(Step::Introduction {
        world: path.target().world().next().unwrap().identity,
        consumed: vec![occurrence],
        value: Construction::new(context::Identity(0), History::default()).literal(label),
    })
    .unwrap()
}

fn restore(path: &Path, value: Fragment<Value>, occurrence: Identity, label: &str) -> Path {
    let world = path.target().world().next().unwrap();
    let marker = world
        .occurrence
        .iter()
        .find(|value| value.value == Value::Atom(label.into()))
        .unwrap()
        .identity;
    path.advance(Step::Historical(admission::Request {
        world: world.identity,
        consumed: vec![marker],
        value,
        witness: vec![Witness {
            state: 0,
            world: world::Identity(0),
            place: model::flow::Place::World(world::Identity(0), occurrence),
        }],
    }))
    .unwrap()
}

#[test]
fn consumption() {
    for changed in [false, true] {
        let initial = super::archive::initial(false);
        let original = super::archive::retain(&initial, Identity(0));
        let replacement = if changed {
            let construction = Construction::new(context::Identity(2), History::default());
            let opened = construction
                .open(construction.definition(original.clone()).unwrap())
                .unwrap();
            construction.rule(opened.input, opened.output).unwrap()
        } else {
            original.clone()
        };
        let consumer = Value::Rule(Box::new(Rule {
            context: context::Identity(0),
            input: Input::new(vec![Particle::new(vec![replacement.value().clone()])]),
            output: Output::new(vec![Destination {
                particle: Particle::new(vec![Value::Atom("D".into())]),
                body: Some(Body::new(context::Identity(0), vec![])),
            }]),
        }));
        let mut world = initial.source().world().cloned().collect::<Vec<_>>();
        world[0].occurrence.push(super::source(20, consumer));
        let mut frame = initial.source().frame().cloned().collect::<Vec<_>>();
        if changed {
            let mut extra = frame[1].clone();
            extra.identity = context::Identity(2);
            extra.held.clear();
            for rule in &mut extra.declaration {
                rule.context = context::Identity(2);
            }
            frame.push(extra);
        }
        let initial = Path::new(
            Configuration::new(context::Identity(0), world, frame, History::default()).unwrap(),
        );
        let consumer = super::archive::retain(&initial, Identity(20));
        let erased = remove(
            &remove(&initial, Identity(0), "First"),
            Identity(20),
            "Second",
        );
        assert_eq!(erased.target().frame().count(), 1);
        let restored = restore(&erased, replacement, Identity(0), "First");
        let path = restore(&restored, consumer, Identity(20), "Second");
        assert_eq!(
            Path::replay(
                path.source().clone(),
                path.record().iter().map(|record| record.step.clone())
            )
            .unwrap(),
            path
        );
        let world = path.target().world().next().unwrap();
        let producer = world.occurrence.iter().find(|value| matches!(&value.value, Value::Rule(rule) if rule.context != context::Identity(0))).unwrap();
        let consumer = world.occurrence.iter().find(|value| matches!(&value.value, Value::Rule(rule) if rule.context == context::Identity(0))).unwrap();
        let request = Request {
            code: Code::Local {
                world: world.identity,
                occurrence: consumer.identity,
            },
            selection: vec![Selection {
                world: world.identity,
                occurrence: vec![producer.identity],
            }],
        };
        let projected = projection::project(&path, request.clone()).unwrap();
        let binding = projected.binding();
        assert_eq!(
            binding.footprint,
            BTreeSet::from([Place::World(world::Identity(0), Identity(0))])
        );
        assert_eq!(binding.exact.is_empty(), changed);
        assert_eq!(
            binding.read,
            BTreeSet::from([Place::World(world::Identity(0), Identity(20))])
        );
        let result = projected.apply().unwrap();
        let site = result.target.world().next().unwrap();
        let frame = result
            .target
            .frame()
            .find(|frame| frame.identity == site.context)
            .unwrap();
        assert_eq!(frame.held.len(), usize::from(!changed));
        assert_eq!(
            site.occurrence
                .iter()
                .any(|value| value.identity == Identity(0)),
            changed
        );
        assert_eq!(
            result.target.frame().count(),
            initial.source().frame().count() + 1
        );
        if !changed {
            assert_eq!(frame.held[0].value, *original.value());
        }
        let inferred = initial
            .advance(Step::Inference {
                path: Box::new(path),
                request,
            })
            .unwrap();
        assert_eq!(inferred.target(), &result.target);
        assert_eq!(inferred.flow(), &result.flow);
    }
}

#[test]
fn resource() {
    for shared in [false, true] {
        let initial = super::archive::initial(shared);
        let value = super::archive::retain(&initial, Identity(0));
        let erased = super::archive::remove(&initial, vec![Identity(0)]);
        let restored = super::archive::restore(&erased, value, 0, Identity(0));
        let frame = restored
            .target()
            .frame()
            .find(|frame| frame.identity != context::Identity(0))
            .unwrap();
        let place = Place::Held(frame.identity, frame.held[0].identity);
        assert_eq!(restored.record()[1].flow.frame[&frame.identity], None);
        assert_eq!(
            restored.flow().frame[&frame.identity],
            Some(context::Identity(1))
        );
        let mut expected = BTreeSet::from([Place::Held(context::Identity(1), Identity(10))]);
        if shared {
            expected.insert(Place::World(world::Identity(0), Identity(10)));
        }
        assert_eq!(restored.flow().resource[&place], expected);
        assert_eq!(
            restored.record()[1].flow.resource[&place].is_empty(),
            !shared
        );
        let code = restored
            .target()
            .world()
            .next()
            .unwrap()
            .occurrence
            .iter()
            .find(|value| matches!(value.value, Value::Rule(_)))
            .unwrap();
        let identity = code.identity;
        let copied = super::archive::retain(&restored, identity);
        let erased = super::archive::remove(&restored, vec![identity]);
        let result = super::archive::restore(&erased, copied, 2, identity);
        let frame = result
            .target()
            .frame()
            .find(|frame| frame.identity != context::Identity(0))
            .unwrap();
        assert_eq!(
            result.flow().frame[&frame.identity],
            Some(context::Identity(1))
        );
        assert_eq!(
            result.flow().resource[&Place::Held(frame.identity, frame.held[0].identity)],
            expected
        );
        assert_eq!(
            result.record()[3].archive[&frame.identity].address,
            model::support::Address::default()
        );
    }
}
