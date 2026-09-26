use super::{Flow, Place, Store};
use crate::basis::Set;
use std::sync::Arc;

fn parent(offset: usize) -> Arc<Flow> {
    Arc::new(Flow {
        resource: (0..32)
            .map(|position| {
                (
                    Place::World(0, position),
                    (0..8)
                        .map(|token| Place::Held(position + offset, token))
                        .collect(),
                )
            })
            .collect(),
        context: (0..32)
            .map(|position| (position + offset..position + offset + 8).collect())
            .collect(),
        frame: vec![Some(offset), None],
    })
}

#[test]
fn composition() {
    let event = Flow {
        resource: (0..32)
            .map(|position| {
                (
                    Place::Held(position, 0),
                    (0..8 + position % 24)
                        .map(|token| Place::World(0, token))
                        .collect::<Set<_>>(),
                )
            })
            .collect(),
        context: (0..32)
            .map(|position| (0..8 + position % 24).collect())
            .collect(),
        frame: vec![Some(1), Some(0), None],
    };
    let event = Arc::new(event);
    for capacity in [0, 128, 65536] {
        let mut store = Store::new(capacity);
        for offset in 0..16 {
            let parent = parent(offset);
            let expected = parent.compose(&event);
            for _ in 0..4 {
                assert_eq!(store.compose(&parent, &event), expected);
            }
            assert_eq!(Arc::strong_count(&parent), 1);
            assert!(store.retained() <= capacity);
        }
        store.evict();
        assert_eq!(store.retained(), 0);
        let parent = parent(99);
        assert_eq!(store.compose(&parent, &event), parent.compose(&event));
        assert_eq!(store.retained(), 0);
    }
}

#[test]
fn frame() {
    use crate::location::Location;
    use crate::program::Symbol;
    use crate::slot::Slot;
    use crate::state::{Frame, State, Token, World};

    let world = |frame, id| {
        Arc::new(World {
            frame,
            particle: vec![Token {
                id,
                value: Symbol::Atom(0),
                capture: None,
            }],
        })
    };
    let scope = |scope, parent| {
        Arc::new(Frame {
            scope,
            parent,
            lexical: parent,
            particle: Default::default(),
            held: Vec::new(),
        })
    };
    let state = State {
        world: vec![world(0, 0), world(1, 1)].into(),
        frame: vec![scope(0, None), scope(1, Some(0))].into(),
    };
    let slot = |world, id| Slot {
        location: Location::World(world),
        token: vec![id],
        position: 0,
    };
    assert!(super::Binding::select(&state, &[slot(0, 0)], 0, Set::default()).is_some());
    assert!(super::Binding::select(&state, &[slot(1, 1)], 1, Set::default()).is_some());
    assert!(super::Binding::select(&state, &[slot(1, 1)], 0, Set::default()).is_none());
    assert!(super::Binding::select(&state, &[slot(0, 0), slot(1, 1)], 0, Set::default()).is_none());
}
