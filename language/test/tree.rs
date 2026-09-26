use super::fragment::{cached, compare};
use super::{Join, Request, Store, Strategy, Traversal};
use crate::index::Index;
use crate::plan::Input;
use crate::program::Program;
use crate::state::State;
use std::sync::Arc;

fn pair(program: &Program, index: &Index, store: &Arc<Store>, depth: [usize; 2]) -> (Join, Join) {
    let input = Input::new(&program.rule[0].input);
    let mut actual = Join::planned(Request {
        input: &input,
        index,
        frame: 0,
        owner: 0,
        store,
    });
    actual.traversal = Join::product(
        &actual.space,
        &actual.order,
        Strategy::Layered {
            previous: depth[0],
            depth: depth[1],
        },
    )
    .unwrap();
    let expected = Join::new(input.pattern(0), index, 0);
    (actual, expected)
}

#[test]
fn boundary() {
    for particle in [["A"; 8].join("."), "A.B.C.D.E.F.G.H.([X] Y)".to_owned()] {
        let program = Program::new(
            &frontend::lowering::parse(&format!(
                "{particle},{particle},I,I,J,J,K,K, [{particle},I,J,K] Done"
            ))
            .unwrap(),
        );
        let index = Index::new(Arc::new(State::initial(&program)));
        for depth in [[0, 1], [0, 2], [1, 2]] {
            for offset in 0..192 {
                let store = Arc::new(Store::new(65536));
                let (mut actual, mut expected) = pair(&program, &index, &store, depth);
                for _ in 0..3 {
                    assert!(compare(&mut actual, &mut expected, &index, 10000));
                    assert!(cached(&actual) > 0);
                    actual.reset(&index);
                    expected.reset(&index);
                }
                compare(&mut actual, &mut expected, &index, offset);
                actual.evict();
                assert_eq!(cached(&actual), 0);
                assert!(compare(&mut actual, &mut expected, &index, 10000));
                drop(actual);
                store.evict();
                assert!(store.budget().reserve(65536));
                store.budget().release(65536);
            }
        }
    }
}

#[test]
fn mutation() {
    let particle = ["A"; 8].join(".");
    let requirement = ["Q"; 8].join(".");
    let initial = ["Q"; 7].join(".");
    let program = Program::new(&frontend::lowering::parse(&format!(
        "{particle},{particle},P,P,P,{initial},{initial},{initial},{initial},R,R,R,R,R,S,S,S,S,S,S, [{particle},P,{requirement},R,S] Done"
    )).unwrap());
    for capacity in [0, 1, 32, 128, 4096, 65536] {
        let mut state = State::initial(&program);
        let mut index = Index::new(Arc::new(state.clone()));
        let store = Arc::new(Store::new(capacity));
        let (mut actual, mut expected) = pair(&program, &index, &store, [0, 1]);
        for iteration in 0..48 {
            compare(&mut actual, &mut expected, &index, iteration % 127);
            if iteration % 7 == 0 {
                actual.evict();
            }
            assert!(compare(&mut actual, &mut expected, &index, 100000));
            actual.reset(&index);
            expected.reset(&index);
            compare(&mut actual, &mut expected, &index, iteration % 97);
            let depth = [1, 2, 0, 2][iteration % 4];
            let removed = actual.space.domain[actual.order[depth]]
                .iter()
                .take(1)
                .map(|member| index.world(member.site))
                .chain(
                    (iteration % 9 == 0)
                        .then(|| index.world(actual.space.domain[actual.order[3]][0].site)),
                )
                .collect::<crate::basis::Set<_>>();
            for &position in removed.iter().rev() {
                let mut world = (*state.world.remove(position)).clone();
                for token in &mut world.particle {
                    token.id += 10000;
                }
                let value = world.particle[0].value;
                if value == program.rule[0].input[2][0] {
                    if world.particle.len() == 7 {
                        let mut token = world.particle[0].clone();
                        token.id += 1000;
                        world.particle.push(token);
                    } else {
                        world.particle.pop();
                    }
                }
                state.world.push(world.into());
            }
            index.advance(Arc::new(state.clone()), &removed);
            actual.advance(&index);
            expected.advance(&index);
            assert_eq!(actual.retained(), actual.size());
        }
        assert!(compare(&mut actual, &mut expected, &index, 100000));
        drop(actual);
        store.evict();
        assert!(store.budget().reserve(capacity));
        store.budget().release(capacity);
    }
}

#[test]
fn saturation() {
    let particle = ["A"; 8].join(".");
    let pattern = ["B"; 12].join(",");
    let companion = ["B.C.E"; 13].join(",");
    let program = Program::new(
        &frontend::lowering::parse(&format!(
            "P,P,{particle},{particle},{companion},C, [P,{particle},{pattern},B.E.E,C] Done"
        ))
        .unwrap(),
    );
    let index = Index::new(Arc::new(State::initial(&program)));
    let store = Arc::new(Store::new(65536));
    let (mut actual, mut expected) = pair(&program, &index, &store, [0, 1]);
    for _ in 0..3 {
        assert!(compare(&mut actual, &mut expected, &index, 3000000));
        actual.reset(&index);
        expected.reset(&index);
    }
    drop(actual);
    store.evict();
    assert!(store.budget().reserve(65536));
    store.budget().release(65536);
}

#[test]
fn activation() {
    let particle = ["A"; 8].join(".");
    let program = Program::new(
        &frontend::lowering::parse(&format!(
            "{particle},{particle},I,I,I,J,J,J,J,K,K,K,K,K, [{particle},I,J,K] Done"
        ))
        .unwrap(),
    );
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let input = Input::new(&program.rule[0].input);
    let store = Arc::new(Store::new(65536));
    let mut actual = Join::planned(Request {
        input: &input,
        index: &index,
        frame: 0,
        owner: 0,
        store: &store,
    });
    let mut expected = Join::new(input.pattern(0), &index, 0);
    for iteration in 0..16 {
        assert!(compare(&mut actual, &mut expected, &index, 10000));
        let depth = usize::from(iteration > 3 && iteration % 2 == 0);
        let position = index.world(actual.space.domain[actual.order[depth]][0].site);
        let world = state.world.remove(position);
        state.world.push(world);
        index.advance(
            Arc::new(state.clone()),
            &crate::basis::Set::single(position),
        );
        actual.advance(&index);
        expected.advance(&index);
        if iteration > 3 {
            assert!(matches!(actual.traversal, Traversal::Layered(_)));
        }
    }
}

#[test]
fn context() {
    let pattern = ["A"; 8].join(".");
    let particle = ["A"; 12].join(".");
    for source in [
        format!("{particle},I,I,J,J,K,K, [{pattern},I,J,K] Done"),
        format!("{pattern}.B,{pattern}.C,B,B,C,C,D,D,D,D, [{pattern},B,C,D] Done"),
    ] {
        let program = Program::new(&frontend::lowering::parse(&source).unwrap());
        let index = Index::new(Arc::new(State::initial(&program)));
        let store = Arc::new(Store::new(65536));
        let (mut actual, mut expected) = pair(&program, &index, &store, [1, 2]);
        for _ in 0..3 {
            assert!(compare(&mut actual, &mut expected, &index, 100000));
            assert!(cached(&actual) > 0);
            assert!(cached(&actual) < 512);
            actual.reset(&index);
            expected.reset(&index);
        }
        drop(actual);
        store.evict();
        assert!(store.budget().reserve(65536));
        store.budget().release(65536);
    }
}

#[test]
fn depth() {
    let particle = ["A"; 8].join(".");
    let suffix = (0..16)
        .map(|position| format!("P{position}"))
        .collect::<Vec<_>>()
        .join(",");
    let program = Program::new(
        &frontend::lowering::parse(&format!("{particle},{suffix}, [{particle},{suffix}] Done"))
            .unwrap(),
    );
    let index = Index::new(Arc::new(State::initial(&program)));
    let store = Arc::new(Store::new(65536));
    let (mut actual, mut expected) = pair(&program, &index, &store, [0, 1]);
    for depth in 0..16 {
        assert!(compare(&mut actual, &mut expected, &index, 1000));
        let Traversal::Layered(product) = &mut actual.traversal else {
            unreachable!()
        };
        product.prefix.update(super::tree::Update {
            index: &index,
            order: &actual.order,
            changed: &[actual.order[depth]],
            depth: Some(depth),
        });
        actual.reset(&index);
        expected.reset(&index);
    }
    assert!(compare(&mut actual, &mut expected, &index, 1000));
    drop(actual);
    store.evict();
    assert!(store.budget().reserve(65536));
    store.budget().release(65536);
}
