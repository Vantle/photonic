use model::application::{Code, Request, Selection};
use model::configuration::Configuration;
use model::context;
use model::path::{Path, Step};
use model::projection;
use model::structure::Value;
use std::collections::{BTreeMap, BTreeSet};

fn code(state: &Configuration, output: &str) -> Code {
    let frame = state
        .frame()
        .find(|frame| frame.identity == context::Identity(0))
        .unwrap();
    let position = frame
        .declaration
        .iter()
        .position(|rule| {
            rule.output.destination().iter().any(|destination| {
                destination
                    .particle
                    .value()
                    .iter()
                    .any(|value| value == &Value::Atom(output.into()))
            })
        })
        .unwrap();
    Code::Declaration {
        context: frame.identity,
        position,
    }
}

fn request(state: &Configuration, code: Code, input: &[&[&str]]) -> Request {
    let mut used = BTreeSet::new();
    let selection = input
        .iter()
        .map(|particle| {
            let world = state
                .world()
                .find(|world| {
                    !used.contains(&world.identity)
                        && particle.iter().all(|label| {
                            world
                                .occurrence
                                .iter()
                                .any(|value| value.value == Value::Atom((*label).into()))
                        })
                })
                .unwrap();
            used.insert(world.identity);
            let mut selected = BTreeSet::new();
            let occurrence = particle
                .iter()
                .map(|label| {
                    let identity = world
                        .occurrence
                        .iter()
                        .find(|value| {
                            !selected.contains(&value.identity)
                                && value.value == Value::Atom((*label).into())
                        })
                        .unwrap()
                        .identity;
                    selected.insert(identity);
                    identity
                })
                .collect();
            Selection {
                world: world.identity,
                occurrence,
            }
        })
        .collect();
    Request { code, selection }
}

fn check(source: &str, path: &Path, request: Request, output: &str) {
    let projection = projection::project(path, request).unwrap();
    let applied = projection.apply().unwrap();
    let binding = projection.binding();
    let footprint = binding
        .footprint
        .iter()
        .map(|&place| super::address(path.source(), place))
        .collect::<BTreeSet<_>>();
    let exact = binding
        .exact
        .iter()
        .map(|&place| super::address(path.source(), place))
        .collect::<BTreeSet<_>>();
    let read = binding
        .read
        .iter()
        .map(|&place| super::address(path.source(), place))
        .collect::<BTreeSet<_>>();
    let mut runtime = photonic::runtime::Runtime::new(photonic::lowering::parse(source).unwrap());
    runtime.run(100_000, Some(photonic::runtime::Limit::default()));
    let snapshot = runtime.snapshot();
    assert!(snapshot.closed, "{source}");
    let witness = snapshot
        .event
        .iter()
        .find(|event| {
            if event.source != 0 || event.status != photonic::support::Status::Supported {
                return false;
            }
            let target = &snapshot.state[event.target];
            if !target
                .world
                .iter()
                .any(|world| world.particle.iter().any(|value| value.label == output))
            {
                return false;
            }
            let source = &snapshot.state[0];
            event
                .footprint
                .iter()
                .map(|&place| super::label(source, place))
                .collect::<BTreeSet<_>>()
                == footprint
                && event
                    .exact
                    .iter()
                    .map(|&place| super::label(source, place))
                    .collect::<BTreeSet<_>>()
                    == exact
                && event
                    .read
                    .iter()
                    .map(|&place| super::label(source, place))
                    .collect::<BTreeSet<_>>()
                    == read
        })
        .unwrap_or_else(|| panic!("native binding missing for {source}: {binding:?}"));
    let target = &snapshot.state[witness.target];
    super::compare(&applied.target, target);
    let expected = applied
        .flow
        .resource
        .iter()
        .map(|(&place, basis)| {
            (
                super::address(&applied.target, place),
                basis
                    .iter()
                    .map(|&place| super::address(path.source(), place))
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert!(snapshot.view.iter().any(|view| {
        view.source == 0
            && view.target == witness.target
            && view.status == photonic::support::Status::Supported
            && view
                .resource
                .iter()
                .map(|link| {
                    (
                        super::label(target, link.target),
                        link.source
                            .iter()
                            .map(|&place| super::label(&snapshot.state[0], place))
                            .collect::<BTreeSet<_>>(),
                    )
                })
                .collect::<BTreeMap<_, _>>()
                == expected
            && view.context == vec![vec![0]]
            && view.frame == vec![Some(0)]
    }));
}

#[test]
fn projection() {
    for (source, producer, consumed, input, exact) in [
        (
            "A [A] B [B] D",
            "B",
            vec![&["A"][..]],
            vec![&["B"][..]],
            false,
        ),
        (
            "A [A] A [A] D",
            "A",
            vec![&["A"][..]],
            vec![&["A"][..]],
            true,
        ),
        (
            "A.Keep [A] B,C [B,C] D",
            "B",
            vec![&["A"][..]],
            vec![&["B"][..], &["C"][..]],
            false,
        ),
        (
            "A.B [A.B] C [C] D",
            "C",
            vec![&["A", "B"][..]],
            vec![&["C"][..]],
            false,
        ),
    ] {
        let initial = super::program::build(source);
        let consumer = code(&initial, "D");
        let step = request(&initial, code(&initial, producer), &consumed);
        let path = Path::new(initial).advance(Step::Application(step)).unwrap();
        let request = request(path.target(), consumer, &input);
        let projected = projection::project(&path, request.clone()).unwrap();
        assert_eq!(projected.binding().exact.is_empty(), !exact);
        assert_eq!(projected.binding().world.len(), 1);
        check(source, &path, request, "D");
    }
}

#[test]
fn generated() {
    let source = "Seed.A [Seed] [A] B";
    let initial = super::program::build(source);
    let step = request(&initial, super::declared(&initial, 0, "Seed"), &[&["Seed"]]);
    let path = Path::new(initial).advance(Step::Application(step)).unwrap();
    let world = path.target().world().next().unwrap();
    let occurrence = world
        .occurrence
        .iter()
        .find(|value| matches!(&value.value, Value::Rule(_)))
        .unwrap()
        .identity;
    let request = request(
        path.target(),
        Code::Local {
            world: world.identity,
            occurrence,
        },
        &[&["A"]],
    );
    let projected = projection::project(&path, request.clone()).unwrap();
    assert_eq!(projected.binding().owner, Some(context::Identity(0)));
    assert_eq!(
        projected
            .binding()
            .read
            .iter()
            .map(|&place| super::address(path.source(), place))
            .collect::<Vec<_>>(),
        vec!["0:Seed"]
    );
    assert_eq!(projected.binding().exact, projected.binding().footprint);
    check(source, &path, request, "B");
}

#[test]
fn imported() {
    let source = "Enter.Make.Call [Enter] ([Make] [Call] X)";
    let initial = super::program::build(source);
    let step = request(
        &initial,
        super::declared(&initial, 0, "Enter"),
        &[&["Enter"]],
    );
    let path = Path::new(initial).advance(Step::Application(step)).unwrap();
    let step = request(
        path.target(),
        super::declared(path.target(), 1, "Make"),
        &[&["Make"]],
    );
    let path = path.advance(Step::Application(step)).unwrap();
    let world = path.target().world().next().unwrap();
    let occurrence = world
        .occurrence
        .iter()
        .find(|value| matches!(&value.value, Value::Rule(_)))
        .unwrap()
        .identity;
    let request = request(
        path.target(),
        Code::Local {
            world: world.identity,
            occurrence,
        },
        &[&["Call"]],
    );
    let projected = projection::project(&path, request.clone()).unwrap();
    assert_eq!(projected.binding().owner, None);
    assert_eq!(
        projected
            .binding()
            .read
            .iter()
            .map(|&place| super::address(path.source(), place))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["0:Enter".into(), "0:Make".into()])
    );
    assert_eq!(projected.binding().exact, projected.binding().footprint);
    check(source, &path, request, "X");
}

#[test]
fn scoped() {
    for exact in [false, true] {
        let source = if exact {
            "A.Keep [A] A [A] (D [Never] E)"
        } else {
            "A.Keep [A] B [B] (D [Never] E)"
        };
        let initial = super::program::build(source);
        let step = request(
            &initial,
            code(&initial, if exact { "A" } else { "B" }),
            &[&["A"]],
        );
        let consumer = code(&initial, "D");
        let path = Path::new(initial).advance(Step::Application(step)).unwrap();
        let projected = projection::project(
            &path,
            request(path.target(), consumer, &[&[if exact { "A" } else { "B" }]]),
        )
        .unwrap();
        let result = projected.apply().unwrap();
        let mut runtime =
            photonic::runtime::Runtime::new(photonic::lowering::parse(source).unwrap());
        runtime.run(10_000, Some(photonic::runtime::Limit::default()));
        let snapshot = runtime.snapshot();
        let event = snapshot
            .event
            .iter()
            .find(|event| {
                event.source == 0
                    && event.status == photonic::support::Status::Supported
                    && event.exact.is_empty() != exact
                    && snapshot.state[event.target]
                        .world
                        .iter()
                        .any(|world| world.particle.iter().any(|value| value.label == "D"))
            })
            .unwrap();
        let target = &snapshot.state[event.target];
        super::compare(&result.target, target);
        assert_eq!(target.frame.len(), 2);
        assert_eq!(target.frame[1].held.len(), usize::from(exact));
        assert_eq!(target.frame[1].parent, Some(0));
        assert_eq!(target.frame[1].lexical, Some(0));
        let expected = result
            .flow
            .resource
            .iter()
            .map(|(&place, basis)| {
                let label = match place {
                    model::flow::Place::World(_, _) => super::address(&result.target, place),
                    model::flow::Place::Held(_, _) => "held:A".into(),
                };
                (
                    label,
                    basis
                        .iter()
                        .map(|&place| super::address(path.source(), place))
                        .collect::<BTreeSet<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert!(snapshot.view.iter().any(|view| {
            view.source == 0
                && view.target == event.target
                && view.status == photonic::support::Status::Supported
                && view.context == vec![vec![0]]
                && view.frame == vec![Some(0), None]
                && view
                    .resource
                    .iter()
                    .map(|link| {
                        let label = match link.target {
                            photonic::flow::Place::World(_, _) => super::label(target, link.target),
                            photonic::flow::Place::Held(_, _) => "held:A".into(),
                        };
                        (
                            label,
                            link.source
                                .iter()
                                .map(|&place| super::label(&snapshot.state[0], place))
                                .collect::<BTreeSet<_>>(),
                        )
                    })
                    .collect::<BTreeMap<_, _>>()
                    == expected
        }));
    }
}

#[test]
fn escaping() {
    let source = "Enter.Make.Call [Enter] ([A] Local, [Make] [Call] (A [Local] Done),) [A] Global";
    let initial = super::program::build(source);
    let step = request(
        &initial,
        super::declared(&initial, 0, "Enter"),
        &[&["Enter"]],
    );
    let path = Path::new(initial).advance(Step::Application(step)).unwrap();
    let step = request(
        path.target(),
        super::declared(path.target(), 1, "Make"),
        &[&["Make"]],
    );
    let path = path.advance(Step::Application(step)).unwrap();
    let world = path.target().world().next().unwrap();
    let occurrence = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap()
        .identity;
    let projected = projection::project(
        &path,
        request(
            path.target(),
            Code::Local {
                world: world.identity,
                occurrence,
            },
            &[&["Call"]],
        ),
    )
    .unwrap();
    let applied = projected.apply().unwrap();
    assert_eq!(applied.target.frame().count(), 3);
    let local = super::step(
        &applied.target,
        super::declared(&applied.target, 1, "A"),
        "A",
    );
    let done = super::step(
        &local.target,
        super::declared(&local.target, 2, "Local"),
        "Local",
    );
    let mut runtime = photonic::runtime::Runtime::new(photonic::lowering::parse(source).unwrap());
    runtime.run(10_000, Some(photonic::runtime::Limit::default()));
    let snapshot = runtime.snapshot();
    let event = snapshot
        .event
        .iter()
        .find(|event| {
            event.source == 0
                && event.status == photonic::support::Status::Supported
                && event.read.len() == 2
                && snapshot.state[event.target].frame.len() == 3
                && snapshot.state[event.target]
                    .world
                    .iter()
                    .any(|world| world.particle.iter().any(|value| value.label == "A"))
        })
        .unwrap();
    super::compare(&applied.target, &snapshot.state[event.target]);
    let target = snapshot
        .state
        .iter()
        .find(|state| {
            state.status == photonic::support::Status::Supported
                && state.frame.len() == done.target.frame().count()
                && state.world.len() == 1
                && state.world[0].particle.len() == 3
                && state.world[0]
                    .particle
                    .iter()
                    .any(|value| value.label == "Done")
        })
        .unwrap();
    super::compare(&done.target, target);
}

#[test]
fn coalescence() {
    let source = "A.Call [A] A.A [A.A] (Make [Make] [Call] [Payload] Result)";
    let initial = super::program::build(source);
    let step = request(&initial, code(&initial, "A"), &[&["A"]]);
    let entering = code(&initial, "Make");
    let path = Path::new(initial).advance(Step::Application(step)).unwrap();
    let step = request(path.target(), entering, &[&["A", "A"]]);
    let path = path.advance(Step::Application(step)).unwrap();
    let step = request(
        path.target(),
        super::declared(path.target(), 1, "Make"),
        &[&["Make"]],
    );
    let path = path.advance(Step::Application(step)).unwrap();
    let world = path.target().world().next().unwrap();
    let occurrence = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap()
        .identity;
    let projected = projection::project(
        &path,
        request(
            path.target(),
            Code::Local {
                world: world.identity,
                occurrence,
            },
            &[&["Call"]],
        ),
    )
    .unwrap();
    let result = projected.apply().unwrap();
    let held = &result
        .target
        .frame()
        .find(|frame| frame.identity != context::Identity(0))
        .unwrap()
        .held;
    assert_eq!(held.len(), 1);
    assert_eq!(
        held[0].identity,
        path.source().world().next().unwrap().occurrence[0].identity
    );
    Configuration::new(
        result.target.root(),
        result.target.world().cloned().collect(),
        result.target.frame().cloned().collect(),
        result.target.history().clone(),
    )
    .unwrap();
}
