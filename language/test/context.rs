use super::{Context, scan};
use crate::change::Change;
use crate::index::Index;
use crate::program::Program;
use crate::state::State;
use std::sync::Arc;

#[test]
fn mutation() {
    let program = Program::new(crate::lowering::parse("A [A] B").unwrap());
    let mut state = State::initial(&program);
    let root = state.frame[0].clone();
    let world = state.world[0].clone();
    for frame in 1..96 {
        let mut value = (*root).clone();
        value.lexical = Some((frame - 1) / 2);
        state.frame.push(value.into());
        let mut value = (*world).clone();
        value.frame = frame;
        value.particle[0].id = frame;
        state.world.push(value.into());
    }
    let mut index = Index::new(Arc::new(state.clone()));
    let mut context = Context::default();
    for iteration in 0..512 {
        let previous = index.state.clone();
        let frame = 1 + iteration % 95;
        Arc::make_mut(&mut state.frame[frame]).lexical =
            (iteration % 3 != 0).then_some(iteration % frame);
        let changed = vec![frame, iteration % 7];
        index.update(
            Arc::new(state.clone()),
            &Change {
                world: Default::default(),
                insertion: state.world.len()..state.world.len(),
                frame: changed.clone(),
            },
        );
        assert_eq!(
            context.select(&index, &previous, &changed),
            scan(&index, &changed)
        );
        let forest = context.forest.as_ref().unwrap();
        assert_eq!(
            forest.retained,
            forest
                .child
                .iter()
                .map(crate::membership::Set::len)
                .sum::<usize>()
        );
        if iteration % 13 == 0 {
            context.evict();
            assert_eq!(context.retained(), 0);
            assert_eq!(
                context.select(&index, &previous, &changed),
                scan(&index, &changed)
            );
            context = Context::default();
        }
    }
}

#[test]
fn resizing() {
    let program = Program::new(crate::lowering::parse("A [A] B").unwrap());
    let initial = State::initial(&program);
    let mut index = Index::new(Arc::new(initial.clone()));
    let mut context = Context::default();
    for width in [1, 40, 80, 48, 16, 64, 1, 96] {
        let previous = index.state.clone();
        let mut state = initial.clone();
        state.frame = vec![initial.frame[0].clone(); width].into();
        state.world.clear();
        for frame in 0..width {
            if frame > 0 {
                Arc::make_mut(&mut state.frame[frame]).lexical = Some((frame - 1) / 2);
            }
            let mut world = (*initial.world[0]).clone();
            world.frame = frame;
            world.particle[0].id = frame;
            state.world.push(world.into());
        }
        let changed = (0..previous.frame.len().max(width)).collect::<Vec<_>>();
        index.update(
            Arc::new(state),
            &Change {
                world: (0..previous.world.len()).collect(),
                insertion: 0..width,
                frame: changed.clone(),
            },
        );
        assert_eq!(
            context.select(&index, &previous, &changed),
            scan(&index, &changed)
        );
        if let Some(forest) = &context.forest {
            assert_eq!(forest.retained, width - 1);
        } else {
            assert!(width < 32);
        }
    }
}
