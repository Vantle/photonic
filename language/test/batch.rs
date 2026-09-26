use super::playback::Playback;
use super::slot::Slot;
use super::trace::Trace;
use crate::budget::Account;
use std::task::Poll;

#[test]
fn boundary() {
    let mut trace = Trace::new(&Account::new(4096), 1).unwrap();
    for position in 0..3 {
        for _ in 0..[7, 11, 1][position] {
            assert!(trace.append(&Poll::Pending, 0.., 4096));
        }
        assert!(trace.append(
            &Poll::Ready(Some(vec![Slot {
                site: position,
                position: 0,
                token: vec![position],
            }])),
            0..,
            4096
        ));
    }
    assert!(trace.append(&Poll::Ready(None), 0.., 4096));
    for offset in 0..=trace.length {
        for length in 0..32 {
            let mut actual = Playback::default();
            actual.seek(&trace, offset);
            let mut expected = Playback::default();
            for _ in 0..offset {
                expected.step(&trace, &[0], &[]);
            }
            let available = actual.waiting(&trace);
            let skipped = actual.skip(&trace, length);
            assert_eq!(skipped, available.min(length));
            for _ in 0..skipped {
                assert_eq!(expected.step(&trace, &[0], &[]), Some(Poll::Pending));
            }
            assert_eq!(actual.cursor, expected.cursor);
            assert_eq!(actual.progress, expected.progress);
            assert_eq!(actual.waiting(&trace), expected.waiting(&trace));
            loop {
                let result = actual.step(&trace, &[0], &[]);
                assert_eq!(result, expected.step(&trace, &[0], &[]));
                if result.is_none() {
                    break;
                }
            }
        }
    }
}
