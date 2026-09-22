use super::{Request, Store};
use crate::factor::Budget;
use crate::index::Index;
use crate::program::{Program, Symbol};
use crate::state::State;
use crate::term::Term;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn scan(index: &Index, pattern: &[Term], frame: usize) -> Vec<usize> {
    index
        .state
        .world
        .iter()
        .enumerate()
        .filter(|(_, world)| {
            world.frame == frame
                && pattern
                    .iter()
                    .all(|term| world.particle.iter().any(|token| term.matches(token)))
        })
        .map(|(position, _)| index.site(position))
        .collect()
}

#[test]
fn mutation() {
    let program = Program::new(crate::lowering::parse(&vec!["A.B,B.C,A.C"; 32].join(",")).unwrap());
    for capacity in [0, 3, 32, 65536] {
        let budget = Arc::new(Budget::new(capacity));
        let accounting = Arc::new(AtomicUsize::new(0));
        let store = Store::new(budget.clone(), accounting.clone());
        let mut state = State::initial(&program);
        let mut index = Index::new(Arc::new(state.clone()));
        let pattern = [
            Term::new(Symbol::Atom(0), None),
            Term::new(Symbol::Atom(1), None),
        ];
        let initial = store.select(Request {
            pattern: pattern.iter().cloned(),
            index: &index,
            frame: 0,
        });
        assert_eq!(initial.site, scan(&index, &pattern, 0));
        for iteration in 0..96 {
            let position = iteration % state.world.len();
            let world = state.world.remove(position);
            state.world.push(world);
            index.advance(
                Arc::new(state.clone()),
                &crate::basis::Set::single(position),
            );
            if let Some(node) = &initial.node {
                let insertion = node.change(&index).insertion;
                let expected = scan(&index, &pattern, 0)
                    .into_iter()
                    .filter(|site| index.insertion.contains(site))
                    .collect::<Vec<_>>();
                assert_eq!(insertion.site, expected);
                let repeated = node.change(&index).insertion;
                assert_eq!(repeated.site, expected);
                if insertion.admitted() {
                    assert!(Arc::ptr_eq(&insertion, &repeated));
                }
                if iteration % 7 == 0 {
                    assert_eq!(node.select(&index).site, scan(&index, &pattern, 0));
                }
            }
            let current = store.select(Request {
                pattern: pattern.iter().cloned(),
                index: &index,
                frame: 0,
            });
            assert_eq!(current.site, scan(&index, &pattern, 0));
            if iteration % 5 == 0 {
                store.evict();
            }
            assert!(accounting.load(Ordering::Relaxed) <= capacity);
        }
        drop(initial);
        store.evict();
        assert_eq!(accounting.load(Ordering::Relaxed), 0);
        assert!(budget.reserve(capacity));
        budget.release(capacity);
    }
}

#[test]
fn identity() {
    let program = Program::new(crate::lowering::parse(&vec!["A.B.([X] Y)"; 40].join(",")).unwrap());
    let mut state = State::initial(&program);
    let budget = Arc::new(Budget::new(65536));
    let accounting = Arc::new(AtomicUsize::new(0));
    let store = Store::new(budget, accounting);
    let pattern = [
        Term::new(Symbol::Atom(0), None),
        Term::new(Symbol::Atom(1), None),
    ];
    let mut index = Index::new(Arc::new(state.clone()));
    let first = store
        .select(Request {
            pattern: pattern.iter().cloned(),
            index: &index,
            frame: 0,
        })
        .node
        .unwrap();
    let permuted = [pattern[1].clone(), pattern[0].clone(), pattern[1].clone()];
    let second = store
        .select(Request {
            pattern: permuted.iter().cloned(),
            index: &index,
            frame: 0,
        })
        .node
        .unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert!(Arc::ptr_eq(&first.select(&index), &second.select(&index)));
    let captured = [Term::new(Symbol::Rule(0), Some(0))];
    let capture = store.select(Request {
        pattern: captured.iter().cloned(),
        index: &index,
        frame: 0,
    });
    assert_eq!(capture.site, scan(&index, &captured, 0));
    assert!(capture.node.is_some());
    let other = [Term::new(Symbol::Rule(0), Some(1))];
    assert!(
        store
            .select(Request {
                pattern: other.iter().cloned(),
                index: &index,
                frame: 0
            })
            .site
            .is_empty()
    );
    assert!(
        store
            .select(Request {
                pattern: pattern.iter().cloned(),
                index: &index,
                frame: 1
            })
            .site
            .is_empty()
    );
    let previous = first.select(&index);
    state.world.remove(0);
    let other = Index::new(Arc::new(state.clone()));
    assert_eq!(first.select(&other).site, scan(&other, &pattern, 0));
    assert_eq!(first.select(&index).site, previous.site);
    index.advance(Arc::new(state), &crate::basis::Set::single(0));
    assert_eq!(first.select(&index).site, scan(&index, &pattern, 0));
    assert!(!Arc::ptr_eq(&previous, &first.select(&index)));
}

#[test]
fn continuity() {
    let program =
        Program::new(crate::lowering::parse(&format!("{},C", vec!["A.B"; 40].join(","))).unwrap());
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let budget = Arc::new(Budget::new(65536));
    let accounting = Arc::new(AtomicUsize::new(0));
    let store = Store::new(budget, accounting);
    let pattern = [
        Term::new(Symbol::Atom(0), None),
        Term::new(Symbol::Atom(1), None),
    ];
    let node = store
        .select(Request {
            pattern: pattern.iter().cloned(),
            index: &index,
            frame: 0,
        })
        .node
        .unwrap();
    let previous = node.select(&index);
    let world = state.world.remove(40);
    state.world.push(world);
    index.advance(Arc::new(state.clone()), &crate::basis::Set::single(40));
    assert!(Arc::ptr_eq(&previous, &node.select(&index)));
    let change = node.change(&index);
    assert!(!change.affected);
    assert!(change.insertion.site.is_empty());
    let world = state.world.remove(0);
    state.world.push(world);
    index.advance(Arc::new(state.clone()), &crate::basis::Set::single(0));
    assert_eq!(node.change(&index).insertion.site, index.insertion);
    let current = node.select(&index);
    assert_eq!(current.site, scan(&index, &pattern, 0));
    assert!(!Arc::ptr_eq(&previous, &current));
    for _ in 0..2 {
        let world = state.world.remove(0);
        state.world.push(world);
        index.advance(Arc::new(state.clone()), &crate::basis::Set::single(0));
    }
    assert_eq!(node.select(&index).site, scan(&index, &pattern, 0));
    assert!(!Arc::ptr_eq(&current, &node.select(&index)));
}

#[test]
fn context() {
    use crate::state::{Frame, Token, World};
    let mut state = State {
        world: (0..96)
            .map(|position| {
                World {
                    frame: position / 48,
                    particle: (0..24)
                        .map(|value| Token {
                            id: position * 25 + value,
                            value: Symbol::Atom(value),
                            capture: None,
                        })
                        .chain(std::iter::once(Token {
                            id: position * 25 + 24,
                            value: Symbol::Rule(0),
                            capture: Some(position % 2),
                        }))
                        .collect(),
                }
                .into()
            })
            .collect(),
        frame: (0..2)
            .map(|_| {
                Frame {
                    scope: 0,
                    parent: None,
                    lexical: None,
                    particle: Vec::new(),
                    held: Vec::new(),
                }
                .into()
            })
            .collect(),
    };
    let mut index = Index::new(Arc::new(state.clone()));
    let store = Store::new(Arc::new(Budget::new(65536)), Arc::new(AtomicUsize::new(0)));
    let mut subscription = Vec::new();
    for frame in 0..2 {
        for capture in 0..2 {
            let pattern = [
                Term::new(Symbol::Atom(0), None),
                Term::new(Symbol::Rule(0), Some(capture)),
            ];
            let node = store
                .select(Request {
                    pattern: pattern.iter().cloned(),
                    index: &index,
                    frame,
                })
                .node
                .unwrap();
            subscription.push((frame, pattern, node));
        }
    }
    for (position, (_, _, node)) in subscription.iter().enumerate() {
        for (_, _, other) in &subscription[..position] {
            assert!(!Arc::ptr_eq(node, other));
        }
    }
    for iteration in 0..128 {
        let position = iteration % state.world.len();
        let mut world = (*state.world.remove(position)).clone();
        world.frame = 1 - world.frame;
        for token in &mut world.particle {
            token.id = 2400 + iteration * 25 + token.id % 25;
            if let Some(capture) = &mut token.capture {
                *capture = 1 - *capture;
            }
        }
        state.world.push(world.into());
        index.advance(
            Arc::new(state.clone()),
            &crate::basis::Set::single(position),
        );
        for (frame, pattern, node) in &subscription {
            let expected = scan(&index, pattern, *frame);
            assert_eq!(
                node.change(&index).insertion.site,
                expected
                    .iter()
                    .copied()
                    .filter(|site| index.insertion.contains(site))
                    .collect::<Vec<_>>()
            );
            assert_eq!(node.select(&index).site, expected);
        }
    }
}

#[test]
fn saturation() {
    let program = Program::new(crate::lowering::parse(&vec!["A.B"; 40].join(",")).unwrap());
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let budget = Arc::new(Budget::new(64));
    let accounting = Arc::new(AtomicUsize::new(0));
    let store = Store::new(budget.clone(), accounting.clone());
    let pattern = [
        Term::new(Symbol::Atom(0), None),
        Term::new(Symbol::Atom(1), None),
    ];
    let node = store
        .select(Request {
            pattern: pattern.iter().cloned(),
            index: &index,
            frame: 0,
        })
        .node
        .unwrap();
    let previous = node.select(&index);
    assert!(previous.admitted());
    let world = state.world.remove(0);
    state.world.push(world);
    index.advance(Arc::new(state), &crate::basis::Set::single(0));
    let current = node.select(&index);
    assert_eq!(current.site, scan(&index, &pattern, 0));
    assert!(!current.admitted());
    assert!(accounting.load(Ordering::Relaxed) <= 64);
    drop(previous);
    assert!(node.select(&index).admitted());
    store.evict();
    drop(node);
    drop(current);
    assert_eq!(accounting.load(Ordering::Relaxed), 0);
    assert!(budget.reserve(64));
}

#[test]
fn concurrency() {
    let program = Program::new(crate::lowering::parse(&vec!["A.B,A.C"; 40].join(",")).unwrap());
    let initial = State::initial(&program);
    let mut reversed = initial.clone();
    reversed.world = initial.world.iter().rev().cloned().collect();
    let budget = Arc::new(Budget::new(128));
    let accounting = Arc::new(AtomicUsize::new(0));
    let store = Store::new(budget.clone(), accounting.clone());
    std::thread::scope(|scope| {
        for state in [initial, reversed] {
            let store = &store;
            let accounting = &accounting;
            scope.spawn(move || {
                let index = Index::new(Arc::new(state));
                let pattern = [
                    Term::new(Symbol::Atom(0), None),
                    Term::new(Symbol::Atom(1), None),
                ];
                let expected = scan(&index, &pattern, 0);
                for iteration in 0..128 {
                    let domain = store.select(Request {
                        pattern: pattern.iter().cloned(),
                        index: &index,
                        frame: 0,
                    });
                    assert_eq!(domain.site, expected);
                    if let Some(node) = domain.node {
                        assert_eq!(node.select(&index).site, expected);
                        assert_eq!(node.change(&index).insertion.site, expected);
                    }
                    if iteration % 17 == 0 {
                        store.evict();
                    }
                    assert!(accounting.load(Ordering::Relaxed) <= 128);
                }
            });
        }
    });
    store.evict();
    assert_eq!(accounting.load(Ordering::Relaxed), 0);
    assert!(budget.reserve(128));
}

#[test]
fn growth() {
    let program = Program::new(crate::lowering::parse(&vec!["A.B"; 40].join(",")).unwrap());
    let mut state = State::initial(&program);
    let original = state.world[0].clone();
    let mut index = Index::new(Arc::new(state.clone()));
    let budget = Arc::new(Budget::new(65536));
    let accounting = Arc::new(AtomicUsize::new(0));
    let store = Store::new(budget.clone(), accounting.clone());
    let pattern = [
        Term::new(Symbol::Atom(0), None),
        Term::new(Symbol::Atom(1), None),
    ];
    let node = store
        .select(Request {
            pattern: pattern.iter().cloned(),
            index: &index,
            frame: 0,
        })
        .node
        .unwrap();
    state.world.extend((0..5000).map(|position| {
        let mut world = (*original).clone();
        for (offset, token) in world.particle.iter_mut().enumerate() {
            token.id = 80 + position * 2 + offset;
        }
        world.into()
    }));
    index.advance(Arc::new(state.clone()), &Default::default());
    let change = node.change(&index);
    assert_eq!(change.insertion.site, index.insertion);
    assert!(!change.insertion.admitted());
    let selection = node.select(&index);
    assert_eq!(selection.site, scan(&index, &pattern, 0));
    assert_eq!(selection.site.len(), 5040);
    assert!(!selection.admitted());
    let removed = (0..state.world.len()).collect();
    state.world = Default::default();
    index.advance(Arc::new(state.clone()), &removed);
    assert!(node.select(&index).site.is_empty());
    state.world.push(original);
    index.advance(Arc::new(state), &Default::default());
    assert_eq!(node.select(&index).site, scan(&index, &pattern, 0));
    assert_eq!(node.select(&index).site.len(), 1);
    drop(selection);
    drop(change);
    drop(node);
    store.evict();
    assert_eq!(accounting.load(Ordering::Relaxed), 0);
    assert!(budget.reserve(65536));
}
