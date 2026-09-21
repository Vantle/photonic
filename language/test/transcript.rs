use super::Search;
use crate::index::Index;
use crate::program::Program;
use crate::selection::Store;
use crate::state::State;
use crate::term::Term;
use std::sync::Arc;
use std::task::Poll;

fn exhaust(mut search: Search) -> Vec<Poll<Option<Vec<crate::slot::Slot>>>> {
    let mut transcript = Vec::new();
    for _ in 0..10000 {
        let result = search.step();
        let complete = matches!(result, Poll::Ready(None));
        transcript.push(result);
        if complete {
            return transcript;
        }
    }
    panic!("fixture did not complete")
}

#[test]
fn identity() {
    use crate::program::Symbol;
    use crate::state::{Token, World};
    let program = Program::new(crate::lowering::parse("A").unwrap());
    let mut state = State::initial(&program);
    state.world = (0..3)
        .map(|position| {
            Arc::new(World {
                frame: 0,
                particle: (0..16)
                    .map(|id| Token {
                        id: position * 16 + id,
                        value: Symbol::Rule(0),
                        capture: Some(position % 2),
                    })
                    .collect(),
            })
        })
        .collect();
    for capacity in [0, 1, 128, 65536] {
        let store = Arc::new(Store::new(capacity));
        for variant in 0..4 {
            let mut state = state.clone();
            match variant {
                1 => state.world.reverse(),
                2 => Arc::make_mut(&mut state.world[0]).particle[0].id += 100,
                3 => Arc::make_mut(&mut state.world[0]).particle[0].capture = Some(1),
                _ => {}
            }
            let index = Arc::new(Index::new(Arc::new(state)));
            for capture in 0..2 {
                let pattern = vec![
                    vec![Term::new(Symbol::Rule(0), Some(0)); 16],
                    vec![Term::new(Symbol::Rule(0), Some(capture)); 16],
                ];
                let expected = exhaust(Search::new(pattern.clone(), index.clone(), 0));
                for _ in 0..2 {
                    let actual = Search::shared(pattern.clone(), index.clone(), 0, &store);
                    assert_eq!(exhaust(actual), expected);
                    assert!(store.transcript().retained() <= capacity);
                }
            }
        }
        store.evict();
        assert_eq!(store.retained(), 0);
    }
}

#[test]
fn projection() {
    let source = format!("{},B,C", ["A"; 32].join("."));
    let program = Program::new(crate::lowering::parse(&source).unwrap());
    let state = State::initial(&program);
    let pattern = state
        .world
        .iter()
        .take(2)
        .map(|world| {
            world
                .particle
                .iter()
                .map(|token| Term::new(token.value, token.capture))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut changed = state.clone();
    Arc::make_mut(&mut changed.world[2]).particle[0].id += 100;
    let index = Arc::new(Index::new(Arc::new(state)));
    let projected = Arc::new(Index::new(Arc::new(changed)));
    let store = Arc::new(Store::new(65536));
    let mut producer = Search::shared(pattern.clone(), index.clone(), 0, &store);
    let mut expected = Vec::new();
    loop {
        let result = producer.step();
        let complete = matches!(result, Poll::Ready(None));
        expected.push(result);
        if complete {
            break;
        }
    }
    assert!(store.retained() > 0);
    for boundary in 0..=expected.len() {
        let mut search = Search::shared(pattern.clone(), projected.clone(), 0, &store);
        assert!(matches!(
            search.replay.as_deref(),
            Some(super::replay::Replay::Playing { .. })
        ));
        for (position, result) in expected.iter().enumerate() {
            if position == boundary {
                search.evict();
            }
            assert_eq!(&search.step(), result);
        }
    }
    let mut search = Search::shared(pattern.clone(), projected.clone(), 0, &store);
    store.evict();
    for result in &expected {
        assert_eq!(&search.step(), result);
    }
    drop(search);
    assert_eq!(store.retained(), 0);
    let mut unfinished = Search::shared(pattern.clone(), projected, 0, &store);
    let _ = unfinished.step();
    drop(unfinished);
    assert_eq!(store.transcript().retained(), 0);
    let mut altered = pattern;
    altered[0].push(Term::new(crate::program::Symbol::Atom(999), None));
    let mut search = Search::shared(altered.clone(), index.clone(), 0, &store);
    let mut reference = Search::new(altered, index, 0);
    for _ in 0..expected.len() {
        assert_eq!(search.step(), reference.step());
    }
}
