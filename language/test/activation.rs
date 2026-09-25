use super::{Cursor, Join, Request, Store, Traversal};
use crate::index::Index;
use crate::plan::Input;
use crate::program::{Program, Symbol};
use crate::state::State;
use std::sync::Arc;
use std::task::Poll;

fn eager(join: &mut Join) {
    if let Traversal::Dormant(width) = join.traversal {
        join.traversal = Traversal::Direct(Box::new(Cursor::new(width)));
    }
}

#[test]
fn boundary() {
    for (source, viable, result) in [
        ("A, [A,A] Done", false, Poll::Pending),
        ("A, [B] Done", false, Poll::Ready(None)),
        ("[] Done", true, Poll::Ready(Some(Vec::new()))),
    ] {
        let program = Program::new(&crate::lowering::parse(source).unwrap());
        let input = Input::new(&program.rule[0].input);
        let index = Index::new(Arc::new(State::initial(&program)));
        let store = Arc::new(Store::new(65536));
        let mut join = Join::planned(Request {
            input: &input,
            index: &index,
            frame: 0,
            owner: 0,
            store: &store,
        });
        for _ in 0..3 {
            assert_eq!(join.viable(), viable);
            assert_eq!(join.step(&index), result);
            assert_eq!(join.retained(), join.size());
            join.reset(&index);
        }
    }
    let program = Program::new(&crate::lowering::parse("A").unwrap());
    let index = Index::new(Arc::new(State::initial(&program)));
    let mut join = Join::new(Arc::new(Vec::new()), &index, 0);
    for _ in 0..3 {
        assert!(join.viable());
        assert_eq!(join.step(&index), Poll::Ready(Some(Vec::new())));
        assert_eq!(join.step(&index), Poll::Ready(None));
        assert_eq!(join.retained(), join.size());
        join.reset(&index);
    }
}

#[test]
fn activation() {
    let particle = ["A"; 8].join(".");
    let source = format!(
        "{},C, [{particle},{particle},B] Done",
        vec![particle.clone(); 40].join(",")
    );
    let program = Program::new(&crate::lowering::parse(&source).unwrap());
    let input = Input::new(&program.rule[0].input);
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let store = Arc::new(Store::new(65536));
    let reference = Arc::new(Store::new(65536));
    let construct = |store| {
        Join::planned(Request {
            input: &input,
            index: &index,
            frame: 0,
            owner: 0,
            store,
        })
    };
    let mut actual = construct(&store);
    let mut expected = construct(&reference);
    assert!(matches!(actual.traversal, Traversal::Dormant(_)));
    eager(&mut expected);
    for iteration in 0..32 {
        assert_eq!(actual.retained(), expected.retained());
        assert_eq!(actual.viable(), expected.viable());
        let mut complete = false;
        for _ in 0..20_000 {
            let result = actual.step(&index);
            assert_eq!(result, expected.step(&index));
            assert_eq!(actual.retained(), expected.retained());
            assert_eq!(actual.retained(), actual.size());
            if result == Poll::Ready(None) {
                complete = true;
                break;
            }
        }
        assert!(complete);
        if iteration % 3 == 0 {
            actual.evict();
            expected.evict();
            store.evict();
            reference.evict();
        }
        let position = state.world.len() - 1;
        let mut world = (*state.world.remove(position)).clone();
        let symbol = if iteration % 2 == 0 { "B" } else { "C" };
        world.particle[0].value = Symbol::Atom(
            program
                .atom
                .iter()
                .position(|value| value == symbol)
                .unwrap(),
        );
        world.particle[0].id += 1;
        state.world.push(world.into());
        index.advance(
            Arc::new(state.clone()),
            &crate::basis::Set::single(position),
        );
        actual.advance(&index);
        expected.advance(&index);
        eager(&mut expected);
    }
}
