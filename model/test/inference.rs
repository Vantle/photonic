use model::application::{Code, Request, Selection};
use model::configuration::Configuration;
use model::context;
use model::path::{Path, Step};
use model::projection;
use model::structure::Value;
use std::collections::BTreeSet;

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
    assert!(
        snapshot.event.iter().any(|event| {
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
        }),
        "native binding missing for {source}: {binding:?}"
    );
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
