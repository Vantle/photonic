use molten::canonical;
use molten::executor::Executor;
use molten::matching::Term;
use molten::program::Symbol;
use molten::runtime::{Limit, Runtime};
use molten::search::Search;
use molten::state::{Frame, State, Token, World};
use std::sync::Arc;
use std::task::Poll;

fn root(world: Vec<World>) -> State {
    State {
        world,
        frame: vec![Frame {
            scope: Arc::new(molten::program::Scope::default()),
            parent: None,
            lexical: None,
            held: Vec::new(),
        }],
    }
}

fn token(id: usize) -> Token {
    Token {
        id,
        value: Symbol::Atom(0),
    }
}

#[test]
fn matching() {
    let state = root(vec![World {
        frame: 0,
        particle: (0..30).map(token).collect(),
    }]);
    let pattern = vec![vec![
        Term {
            value: Symbol::Atom(0),
        };
        15
    ]];
    let mut search = Search::new(pattern, Arc::new(state), 0);
    let mut found = 0;
    for _ in 0..1000 {
        match search.step() {
            Poll::Pending => {}
            Poll::Ready(Some(binding)) => {
                assert_eq!(binding[0].token.len(), 15);
                found += 1;
            }
            Poll::Ready(None) => {
                panic!("155 million combinations cannot be exhausted in 1000 steps")
            }
        }
    }
    assert!(found > 0 && found < 1000);
}

#[test]
fn canonicalization() {
    let state = root(
        (0..30)
            .map(|id| World {
                frame: 0,
                particle: vec![token(id)],
            })
            .collect(),
    );
    let mut search = canonical::Search::new(Arc::new(state));
    for _ in 0..100 {
        assert!(!search.step());
    }
    assert!(search.finish().is_none());
}

#[test]
fn refinement() {
    let mut state = root(
        (0..12)
            .map(|index| World {
                frame: 0,
                particle: vec![token(index), token(index + 1)],
            })
            .collect(),
    );
    state.world[0].particle.push(Token {
        id: 20,
        value: Symbol::Atom(1),
    });
    let mut search = canonical::Search::new(Arc::new(state.clone()));
    let steps = (0..10).find(|_| search.step());
    assert!(
        steps.is_some(),
        "asymmetric sharing should distinguish every coherence"
    );
    let expected = search.finish().unwrap().state;
    state.world.reverse();
    for world in &mut state.world {
        world.particle.reverse();
        for token in &mut world.particle {
            token.id = 100 - token.id;
        }
    }
    assert_eq!(state.canonical().state, expected);
}

#[test]
fn parallel() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../example/reference.json")).unwrap();
    let executor = [
        Executor::new(1).unwrap(),
        Executor::new(2).unwrap(),
        Executor::new(4).unwrap(),
    ];
    for case in fixture
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["closed"] == true)
    {
        let program: molten::source::Program =
            serde_json::from_value(case["program"].clone()).unwrap();
        let mut runtime = Runtime::new(program.clone());
        runtime.run(12000, None);
        assert!(runtime.closed(), "{}", case["name"]);
        let expected = serde_json::to_value(runtime.snapshot()).unwrap();
        for executor in &executor {
            let mut runtime = Runtime::new(program.clone());
            runtime.parallel(executor, 12000, None);
            assert_eq!(
                serde_json::to_value(runtime.snapshot()).unwrap(),
                expected,
                "{}",
                case["name"]
            );
        }
    }
}

#[test]
fn chunking() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../example/reference.json")).unwrap();
    for case in fixture
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["closed"] == true)
    {
        let program: molten::source::Program =
            serde_json::from_value(case["program"].clone()).unwrap();
        let mut complete = Runtime::new(program.clone());
        complete.run(12000, None);
        assert!(complete.closed());
        let expected = serde_json::to_value(complete.snapshot()).unwrap();
        for chunk in [1, 7, 32] {
            let mut runtime = Runtime::new(program.clone());
            for _ in 0..12000 {
                if runtime.closed() {
                    break;
                }
                runtime.run(chunk, None);
            }
            assert_eq!(
                serde_json::to_value(runtime.snapshot()).unwrap(),
                expected,
                "{}",
                case["name"]
            );
        }
    }
}

#[test]
fn fairness() {
    let source = format!(
        "{},Start; [{}] -> Z; [Start] -> Done;",
        vec!["A"; 25].join("."),
        vec!["A"; 12].join(".")
    );
    let mut runtime = Runtime::new(molten::lowering::parse(&source).unwrap());
    runtime.run(
        1000,
        Some(Limit {
            cell: 64,
            ..Limit::default()
        }),
    );
    assert!(!runtime.closed());
    assert!(runtime.snapshot().state.iter().any(|node| {
        node.world
            .iter()
            .any(|world| world.particle.iter().any(|token| token.label == "Done"))
    }));
}

#[test]
fn budget() {
    let source = molten::lowering::parse("Seed.A; [Seed] -> @([A] -> B);").unwrap();
    let mut complete = Runtime::new(source.clone());
    complete.run(12000, None);
    let expected = serde_json::to_value(complete.snapshot()).unwrap();
    for record in [1, complete.record() / 2] {
        let mut runtime = Runtime::new(source.clone());
        runtime.run(
            12000,
            Some(Limit {
                record,
                ..Limit::default()
            }),
        );
        assert!(!runtime.closed());
        if record == 1 {
            assert_eq!(runtime.snapshot().work, 0);
        } else {
            assert!(runtime.snapshot().work > 0);
        }
        runtime.run(12000, Some(Limit::default()));
        assert_eq!(serde_json::to_value(runtime.snapshot()).unwrap(), expected);
    }
}

#[test]
fn initialization() {
    let source = molten::source::Program {
        initial: vec![vec![molten::source::Value::Atom("A".into())]; 100],
        rule: Vec::new(),
    };
    let runtime = Runtime::new(source);
    assert_eq!(runtime.snapshot().state[0].world.len(), 100);
    for mask in 0..64 {
        let initial = (0..3)
            .map(|world| {
                (0..2)
                    .filter(|index| mask & (1 << (world * 2 + index)) != 0)
                    .map(|index| molten::source::Value::Atom(index.to_string()))
                    .collect()
            })
            .collect();
        let program = molten::program::Program::new(molten::source::Program {
            initial,
            rule: Vec::new(),
        });
        let state = State::initial(&program);
        assert_eq!(state, state.canonical().state);
    }
}
