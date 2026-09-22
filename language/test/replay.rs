use super::{Mode, Search};
use crate::index::Index;
use crate::joining::Join;
use crate::program::Program;
use crate::state::State;
use std::sync::Arc;
use std::task::Poll;

fn compare(search: &mut Search, reference: &mut Join, index: &Index, budget: usize) -> usize {
    let mut count = 0;
    for _ in 0..budget {
        let expected = reference.step(index);
        let actual = search.step(index);
        assert_eq!(actual, expected);
        match actual {
            Poll::Ready(None) => return count,
            Poll::Ready(Some(_)) => count += 1,
            Poll::Pending => {}
        }
    }
    count
}

#[test]
fn continuation() {
    for source in [
        "X,A.A,B,B [A,B] C",
        "X,A.A,A.A,A.A [A,A] C",
        "X,A [A.A] B",
        "X,A [ ] B",
    ] {
        let program = Program::new(&crate::lowering::parse(source).unwrap());
        let mut state = State::initial(&program);
        let mut index = Index::new(Arc::new(state.clone()));
        let pattern = crate::plan::Input::new(&program.rule[0].input).pattern(0);
        let mut search = Search::new(pattern.clone(), &index, 0);
        let mut reference = Join::new(pattern, &index, 0);
        for iteration in 0..128 {
            compare(&mut search, &mut reference, &index, iteration % 19);
            search.reset(&index);
            reference.reset(&index);
        }
        compare(&mut search, &mut reference, &index, 1000);
        search.reset(&index);
        reference.reset(&index);
        let symbol = crate::program::Symbol::Atom(program.atom.get_index_of("X").unwrap());
        let removed = state
            .world
            .iter()
            .position(|world| world.particle[0].value == symbol)
            .unwrap();
        let world = state.world.remove(removed);
        state.world.push(world);
        index.advance(Arc::new(state.clone()), &crate::basis::Set::single(removed));
        if program.rule[0].input.is_empty() {
            search.advance(&index);
            reference.advance(&index);
        }
        compare(&mut search, &mut reference, &index, 1000);
        let world = state.world.remove(0);
        state.world.push(world);
        index.advance(Arc::new(state), &crate::basis::Set::single(0));
        search.advance(&index);
        reference.advance(&index);
        compare(&mut search, &mut reference, &index, 1000);
    }
}

#[test]
fn overflow() {
    let source = format!("{} [A] B", vec!["A"; 2048].join("."));
    let program = Program::new(&crate::lowering::parse(&source).unwrap());
    let index = Index::new(Arc::new(State::initial(&program)));
    let pattern = crate::plan::Input::new(&program.rule[0].input).pattern(0);
    let mut search = Search::new(pattern.clone(), &index, 0);
    let mut reference = Join::new(pattern, &index, 0);
    for _ in 0..3 {
        assert_eq!(compare(&mut search, &mut reference, &index, 10000), 2048);
        search.reset(&index);
        reference.reset(&index);
    }
    assert!(matches!(search.mode, Mode::Streaming));
}

#[test]
fn invalidation() {
    let program = Program::new(&crate::lowering::parse("A.X,A.C.C,B,B [A.C,B] Done").unwrap());
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let pattern = crate::plan::Input::new(&program.rule[0].input).pattern(0);
    let mut search = Search::new(pattern.clone(), &index, 0);
    let mut reference = Join::new(pattern, &index, 0);
    for _ in 0..3 {
        compare(&mut search, &mut reference, &index, 1000);
        search.reset(&index);
        reference.reset(&index);
    }
    assert!(matches!(search.mode, Mode::Recording(_)));
    let symbol = crate::program::Symbol::Atom(program.atom.get_index_of("X").unwrap());
    let removed = state
        .world
        .iter()
        .position(|world| world.particle.iter().any(|token| token.value == symbol))
        .unwrap();
    let world = state.world.remove(removed);
    state.world.push(world);
    index.advance(Arc::new(state.clone()), &crate::basis::Set::single(removed));
    search.advance(&index);
    reference.advance(&index);
    assert!(matches!(search.mode, Mode::Recording(_)));
    compare(&mut search, &mut reference, &index, 1000);
    let symbol = crate::program::Symbol::Atom(program.atom.get_index_of("C").unwrap());
    let removed = state
        .world
        .iter()
        .position(|world| world.particle.iter().any(|token| token.value == symbol))
        .unwrap();
    let world = state.world.remove(removed);
    state.world.push(world);
    index.advance(Arc::new(state), &crate::basis::Set::single(removed));
    search.advance(&index);
    reference.advance(&index);
    assert!(matches!(search.mode, Mode::Dormant));
    compare(&mut search, &mut reference, &index, 1000);
}
