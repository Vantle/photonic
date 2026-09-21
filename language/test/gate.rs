use super::{Gate, Store};
use crate::program::Symbol;
use crate::slot::Slot;
use crate::term::Term;
use std::sync::Arc;

#[test]
fn sharing() {
    let store = Arc::new(Store::new(65536));
    for iteration in 0..16 {
        let pattern = (0..512)
            .map(|position| {
                vec![Term::new(
                    Symbol::Rule(position % (512 - iteration)),
                    Some(position % 3),
                )]
            })
            .collect::<Vec<_>>();
        let mut private = Gate::new(&pattern);
        let mut shared = Gate::new(&pattern);
        shared.share(&store);
        for position in 0..512 {
            let slot = Slot {
                world: position,
                position,
                token: vec![iteration],
            };
            private.enqueue(slot.clone());
            shared.enqueue(slot);
            while private.pending() {
                assert!(shared.pending());
                assert_eq!(private.step(), shared.step());
            }
            assert!(!shared.pending());
            if position == 37 && iteration % 2 == 0 {
                store.evict();
            }
        }
        for position in (0..512).rev() {
            let slot = Slot {
                world: position % 31,
                position,
                token: vec![iteration + 1],
            };
            private.enqueue(slot.clone());
            shared.enqueue(slot);
            for _ in 0..17 {
                assert_eq!(private.pending(), shared.pending());
                assert_eq!(private.step(), shared.step());
            }
        }
        assert!(store.retained() <= 65536);
    }
    store.evict();
    assert_eq!(store.retained(), 0);
}

#[test]
fn saturation() {
    for capacity in [0, 6, 60] {
        let store = Arc::new(Store::new(capacity));
        let pattern = (0..512)
            .map(|position| vec![Term::new(Symbol::Atom(position), None)])
            .collect::<Vec<_>>();
        let mut gate = Gate::new(&pattern);
        gate.share(&store);
        let mut count = 0;
        for position in 0..512 {
            gate.enqueue(Slot {
                world: position,
                position,
                token: vec![position],
            });
            while gate.pending() {
                count += usize::from(gate.step().is_some());
            }
        }
        assert_eq!(count, 1);
        assert!(store.retained() <= capacity);
    }
}
