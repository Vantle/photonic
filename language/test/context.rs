use super::Index;
use crate::program::Program;
use crate::state::State;
use std::sync::Arc;

fn scan(state: &State, changed: &[usize]) -> Vec<usize> {
    (0..state.frame.len())
        .filter(|&frame| {
            std::iter::successors(Some(frame), |&frame| state.frame[frame].lexical)
                .any(|frame| changed.contains(&frame))
        })
        .chain(
            changed
                .iter()
                .copied()
                .filter(|&frame| frame >= state.frame.len()),
        )
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[test]
fn mutation() {
    let program = Program::new(crate::lowering::parse("A [A] B").unwrap());
    let mut state = State::initial(&program);
    let root = state.frame[0].clone();
    for frame in 1..96 {
        let mut value = (*root).clone();
        value.lexical = Some((frame - 1) / 2);
        state.frame.push(value.into());
    }
    let mut index = Index::new(&state);
    for iteration in 0..512 {
        let previous = state.clone();
        let frame = 1 + iteration % 95;
        Arc::make_mut(&mut state.frame[frame]).lexical =
            (iteration % 3 != 0).then_some(iteration % frame);
        let changed = vec![frame, iteration % 7];
        index.update(&previous, &state, &changed);
        assert_eq!(index.select(&changed), scan(&state, &changed));
        assert_eq!(
            index.count,
            index
                .child
                .iter()
                .map(crate::membership::Set::len)
                .sum::<usize>()
        );
    }
}

#[test]
fn resizing() {
    let program = Program::new(crate::lowering::parse("A [A] B").unwrap());
    let initial = State::initial(&program);
    let mut state = initial.clone();
    let mut index = Index::new(&state);
    for width in [1, 40, 80, 48, 16, 64, 1, 96] {
        let previous = state.clone();
        state.frame = vec![initial.frame[0].clone(); width].into();
        for frame in 1..width {
            Arc::make_mut(&mut state.frame[frame]).lexical = Some((frame - 1) / 2);
        }
        let changed = (0..previous.frame.len().max(width)).collect::<Vec<_>>();
        index.update(&previous, &state, &changed);
        assert_eq!(index.select(&changed), scan(&state, &changed));
        assert_eq!(index.count, width - 1);
        assert_eq!(index.retained(), width * 2 - 1);
    }
}
