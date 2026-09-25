use crate::agenda::Queue;

#[test]
fn fairness() {
    let mut queue = Queue::new();
    queue.defer(0);
    queue.defer(1);
    for expected in [0, 1] {
        let mut found = false;
        for _ in 0..=4096 {
            queue.push(2);
            let selected = queue.front().copied();
            let value = queue.pop();
            assert_eq!(selected, value);
            if value == Some(expected) {
                found = true;
                break;
            }
        }
        assert!(found);
    }
}

#[test]
fn order() {
    let mut queue = Queue::new();
    queue.extend(0..5);
    queue.defer(5);
    for expected in 0..6 {
        assert_eq!(queue.len(), 6 - expected);
        assert_eq!(queue.front(), Some(&expected));
        assert_eq!(queue.pop(), Some(expected));
    }
    assert!(queue.is_empty());
    assert_eq!(queue.front(), None);
    assert_eq!(queue.pop(), None);
    queue.defer(6);
    assert_eq!(queue.pop(), Some(6));
}
