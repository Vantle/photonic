use super::{Record, Trace};
use crate::factor::Budget;
use crate::joining::playback::Playback;
use crate::slot::Slot;
use std::sync::Arc;
use std::task::Poll;

#[test]
fn lifetime() {
    let budget = Arc::new(Budget::new(4096));
    let mut child = Trace::new(budget.clone(), 3).unwrap();
    let suffix = vec![Slot {
        world: 7,
        position: 0,
        token: vec![11, 13],
    }];
    assert!(child.append(&Poll::Pending, 0.., 4096));
    assert!(child.append(&Poll::Ready(Some(suffix.clone())), 0.., 4096));
    assert!(child.append(&Poll::Ready(None), 0.., 4096));
    let snapshot = child.duplicate(4096).unwrap();
    let prefix = vec![Slot {
        world: 5,
        position: 1,
        token: vec![3],
    }];
    let mut parent = Trace::new(budget.clone(), 2).unwrap();
    assert!(parent.append(&Poll::Pending, 0.., 4096));
    assert!(parent.extend(&child, &prefix, 4096));
    assert_eq!(parent.retained, parent.size());
    assert!(matches!(parent.record[0], Record::Waiting(2)));
    drop(child);
    drop(snapshot);
    let outer = vec![Slot {
        world: 2,
        position: 2,
        token: vec![17],
    }];
    let order = [2, 1, 0];
    let mut playback = Playback::default();
    assert_eq!(playback.skip(&parent, 1), 1);
    assert_eq!(playback.step(&parent, &order, &outer), Some(Poll::Pending));
    let expected = outer
        .into_iter()
        .chain(prefix)
        .chain(suffix)
        .collect::<Vec<_>>();
    assert_eq!(
        playback.step(&parent, &order, &[]),
        Some(Poll::Ready(Some(vec![
            Slot {
                world: 5,
                position: 2,
                token: vec![3]
            },
            Slot {
                world: 7,
                position: 1,
                token: vec![11, 13]
            },
        ])))
    );
    playback.seek(&parent, 2);
    assert_eq!(
        playback.step(&parent, &order, &expected[..1]),
        Some(Poll::Ready(Some(expected)))
    );
    assert_eq!(playback.step(&parent, &order, &[]), None);
    drop(parent);
    assert!(budget.reserve(4096));
}

#[test]
fn refusal() {
    let budget = Arc::new(Budget::new(4096));
    let mut trace = Trace::new(budget.clone(), 1).unwrap();
    assert!(trace.append(&Poll::Pending, 0.., 4096));
    let mut parent = Trace::new(budget.clone(), 1).unwrap();
    assert!(!parent.extend(&trace, &[], 0));
    assert!(parent.record.is_empty());
    assert_eq!(parent.length, 0);
    assert_eq!(parent.retained, parent.size());
    assert!(parent.extend(&trace, &[], 1));
    assert_eq!(parent.retained, parent.size());
    drop(parent);
    drop(trace);
    assert!(budget.reserve(4096));
}
