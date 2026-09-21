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
