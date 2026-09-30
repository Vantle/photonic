use crate::budget::Budget;
use crate::exploration::Plan;
use crate::explored::Explored;
use crate::failure::Code;
use crate::recording::{Engine, Mode};
use crate::render;
use crate::store::Store;

fn plan(source: &str) -> Plan {
    let program = frontend::lowering::parse(source).expect("the example lowers");
    Plan::new(
        &program,
        Mode::Exhaustive,
        Engine::Laser,
        Budget::default(),
        None,
    )
}

fn text(explored: &Explored, configuration: usize) -> String {
    let Explored::Exploration(exploration) = explored else {
        panic!("laser records an exploration");
    };
    render::configuration(exploration, configuration)
}

// A key is a hash, so two programs can share one: each is still explored and answered on its own,
// and a lookup by the shared key refuses rather than answer from either.
#[test]
fn collision() {
    let first = plan("A, [A] B");
    let key = first.key.clone();
    let second = Plan {
        key: key.clone(),
        ..plan("A, [A] C")
    };
    let mut store = Store::default();
    let one = store.explore(first).expect("the first explores");
    let two = store.explore(second).expect("the second explores");
    assert_eq!(text(&one, 1), "B");
    assert_eq!(text(&two, 1), "C");
    let again = store.explore(plan("A, [A] B")).expect("the first is held");
    assert_eq!(text(&again, 1), "B");
    let Err(shared) = store.find(&format!("x{key}")) else {
        panic!("a key two explorations share names neither");
    };
    assert_eq!(shared.code, Code::Exploration);
    assert!(
        shared.message.contains("send the program again"),
        "{shared}"
    );
}

// A key is found by any prefix of at least four digits, written in either case, after exactly one
// leading x.
#[test]
fn lookup() {
    let plan = plan("A, [A] B");
    let key = plan.key.clone();
    let mut store = Store::default();
    store.explore(plan).expect("the program explores");
    for written in [
        format!("x{key}"),
        key.clone(),
        key.to_uppercase(),
        format!("X{}", key.to_uppercase()),
        format!(" x{} ", &key[..4]),
    ] {
        assert!(store.find(&written).is_ok(), "{written}");
    }
    let Err(short) = store.find(&format!("x{}", &key[..3])) else {
        panic!("three digits are too few");
    };
    assert!(short.message.contains("too short"), "{short}");
    for written in [format!("xx{key}"), format!("{key}0"), "x12g4".to_owned()] {
        let Err(refused) = store.find(&written) else {
            panic!("{written} is not a key");
        };
        assert!(
            refused.message.contains("not an exploration key"),
            "{refused}"
        );
    }
}
