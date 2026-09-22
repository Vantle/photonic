use super::Index;
use crate::catalog::Catalog;
use crate::change::Change;
use crate::program::{Program, Symbol};
use crate::reader::Read;
use crate::state::State;
use std::sync::Arc;

fn verify(membership: &mut Index, index: &crate::index::Index, catalog: &Catalog) {
    let state = &index.state;
    let reachable = state.reachable();
    let mut retained = state.frame.len();
    for frame in 0..state.frame.len() {
        let present = reachable.contains(&frame);
        membership.ensure(index, catalog, frame);
        retained += usize::from(present);
        for input in 0..catalog.count() {
            let mut expected = state.frame[frame]
                .particle
                .iter()
                .filter_map(|token| {
                    let Symbol::Rule(rule) = token.value else {
                        return None;
                    };
                    (present && catalog.rule(rule) == input).then_some((
                        rule,
                        token.capture.unwrap(),
                        Read::Context(frame, token.id),
                    ))
                })
                .collect::<Vec<_>>();
            let mut actual = membership
                .select(catalog, index, frame, input)
                .map(|consumer| (consumer.rule, consumer.owner, consumer.read.unwrap()))
                .collect::<Vec<_>>();
            let order = |(rule, owner, read)| {
                let Read::Context(frame, resource) = read else {
                    unreachable!()
                };
                (rule, owner, frame, resource)
            };
            expected.sort_by_key(|value| order(*value));
            actual.sort_by_key(|value| order(*value));
            assert_eq!(actual, expected);
            retained += expected.len() + usize::from(!expected.is_empty());
        }
    }
    assert_eq!(
        membership.retained(),
        if membership.frame.is_some() {
            retained
        } else {
            0
        }
    );
}

#[test]
fn mutation() {
    let program = Program::new(&crate::lowering::parse("A [A] B [A] C [] Seed [B] D").unwrap());
    let catalog = Catalog::new(&program);
    let mut state = State::initial(&program);
    let root = state.frame[0].clone();
    let world = state.world[0].clone();
    state.frame = (0..9)
        .map(|frame| {
            let mut value = (*root).clone();
            value.parent = (frame > 0).then_some(0);
            value.lexical = (frame > 0).then_some(frame / 2);
            value.particle = root
                .particle
                .iter()
                .cloned()
                .map(|mut token| {
                    token.id += frame * 100;
                    token.capture = Some(frame);
                    token
                })
                .collect();
            Arc::new(value)
        })
        .collect();
    state.world = (0..9)
        .map(|frame| {
            let mut value = (*world).clone();
            value.frame = frame;
            value.particle[0].id += frame * 100;
            Arc::new(value)
        })
        .collect();
    let original = state.clone();
    let mut index = crate::index::Index::new(Arc::new(state.clone()));
    let mut membership = Index::new(&index);
    verify(&mut membership, &index, &catalog);
    for iteration in 0..64 {
        let position = iteration % 8 + 1;
        let frame = Arc::make_mut(&mut state.frame[position]);
        match iteration % 5 {
            0 => frame.particle.retain(|token| token.id % 3 != 0),
            1 => frame.particle = original.frame[position].particle.clone(),
            2 => frame.particle = frame.particle.iter().cloned().collect(),
            3 => {
                if !frame.particle.is_empty() {
                    frame.particle[0].capture = Some(0);
                }
            }
            _ => frame.lexical = Some(0),
        }
        let change = Change {
            world: Default::default(),
            insertion: state.world.len()..state.world.len(),
            frame: vec![position],
        };
        index.update(Arc::new(state.clone()), &change);
        membership.advance(&index, &catalog);
        verify(&mut membership, &index, &catalog);
        if iteration == 31 {
            let retained = membership.retained();
            assert_eq!(membership.evict(), retained);
            assert_eq!(membership.evict(), 0);
            verify(&mut membership, &index, &catalog);
        }
    }
}

#[test]
fn retirement() {
    let program = Program::new(&crate::lowering::parse("A [A] B [A] C [] Seed").unwrap());
    let catalog = Catalog::new(&program);
    let original = State::initial(&program);
    let mut index = crate::index::Index::new(Arc::new(original.clone()));
    let mut membership = Index::new(&index);
    verify(&mut membership, &index, &catalog);
    let root = membership.frame.as_ref().unwrap()[0]
        .as_ref()
        .unwrap()
        .group
        .values()
        .next()
        .unwrap()
        .as_ptr();
    for iteration in 0..32 {
        let mut state = original.clone();
        let mut frame = (*state.frame[0]).clone();
        frame.parent = Some(0);
        frame.lexical = Some(0);
        frame.particle = frame
            .particle
            .iter()
            .cloned()
            .map(|mut token| {
                token.id += 100 * (iteration + 1);
                token.capture = Some(1);
                token
            })
            .collect();
        state.frame.push(Arc::new(frame));
        let mut world = (*state.world[0]).clone();
        world.frame = 1;
        world.particle[0].id += 100 * (iteration + 1);
        state.world.push(Arc::new(world));
        let change = Change {
            world: Default::default(),
            insertion: 1..2,
            frame: vec![1],
        };
        index.update(Arc::new(state), &change);
        membership.advance(&index, &catalog);
        verify(&mut membership, &index, &catalog);
        let change = Change {
            world: [1].into_iter().collect(),
            insertion: 1..1,
            frame: vec![1],
        };
        index.update(Arc::new(original.clone()), &change);
        membership.advance(&index, &catalog);
        verify(&mut membership, &index, &catalog);
        assert_eq!(
            membership.frame.as_ref().unwrap()[0]
                .as_ref()
                .unwrap()
                .group
                .values()
                .next()
                .unwrap()
                .as_ptr(),
            root
        );
    }
}
