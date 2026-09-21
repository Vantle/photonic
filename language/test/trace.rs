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

#[test]
fn boundary() {
    for count in [0, 1, 2, 31, 32, 33, 64, 65] {
        let budget = Arc::new(Budget::new(65536));
        let mut trace = Trace::new(budget.clone(), 1).unwrap();
        let expected = (0..count)
            .map(|world| {
                Poll::Ready(Some(vec![Slot {
                    world,
                    position: 0,
                    token: vec![world],
                }]))
            })
            .collect::<Vec<_>>();
        for result in &expected {
            assert!(trace.append(result, 0.., 4096));
        }
        let snapshot = trace.duplicate(4096).unwrap();
        assert!(trace.append(&Poll::Pending, 0.., 4096));
        assert!(trace.append(&Poll::Pending, 0.., 4096));
        assert_eq!(snapshot.length, count);
        assert_eq!(snapshot.retained, snapshot.size());
        drop(trace);
        let mut playback = Playback::default();
        for result in expected {
            assert_eq!(playback.step(&snapshot, &[0], &[]), Some(result));
        }
        assert_eq!(playback.step(&snapshot, &[0], &[]), None);
        drop(snapshot);
        assert!(budget.reserve(65536));
    }
}

#[test]
fn branching() {
    let budget = Arc::new(Budget::new(65536));
    let mut trace = Trace::new(budget.clone(), 1).unwrap();
    for world in 0..128 {
        assert!(trace.append(&Poll::Pending, 0.., 4096));
        assert!(trace.append(
            &Poll::Ready(Some(vec![Slot {
                world,
                position: 0,
                token: vec![world],
            }])),
            0..,
            4096,
        ));
    }
    assert!(trace.append(&Poll::Pending, 0.., 4096));
    let snapshot = trace.duplicate(4096).unwrap();
    assert!(trace.append(&Poll::Pending, 0.., 4096));
    assert!(trace.append(&Poll::Ready(None), 0.., 4096));
    assert_eq!(snapshot.length, 257);
    assert!(!snapshot.complete);
    assert_eq!(trace.length, 258);
    assert_eq!(trace.size(), trace.retained);
    assert_eq!(snapshot.size(), snapshot.retained);
    let mut playback = Playback::default();
    for world in 0..128 {
        assert_eq!(playback.step(&snapshot, &[0], &[]), Some(Poll::Pending));
        assert_eq!(
            playback.step(&snapshot, &[0], &[]),
            Some(Poll::Ready(Some(vec![Slot {
                world,
                position: 0,
                token: vec![world],
            }])))
        );
    }
    assert_eq!(playback.skip(&snapshot, 10), 1);
    assert_eq!(playback.step(&snapshot, &[0], &[]), None);
    drop(trace);
    playback.seek(&snapshot, 255);
    assert!(matches!(
        playback.step(&snapshot, &[0], &[]),
        Some(Poll::Ready(Some(_)))
    ));
    drop(snapshot);
    assert!(budget.reserve(65536));
}
