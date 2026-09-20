use crate::sequence::List;
use std::hash::{DefaultHasher, Hash, Hasher};

fn hash(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn persistence() {
    let mut current = (0..4096).collect::<List<_>>();
    let original = current.clone();
    let mut expected = (0..4096).collect::<Vec<_>>();
    for step in 0..2048 {
        let position = step * 73 % current.len();
        assert_eq!(current.remove(position), expected.remove(position));
        current.push(step);
        expected.push(step);
    }
    assert!(current.iter().eq(expected.iter()));
    assert!(original.iter().copied().eq(0..4096));
    assert!(current.range(40..90).eq(expected[40..90].iter()));
    let mut tree = original;
    tree.truncate(16);
    let flat = (0..16).collect::<List<_>>();
    assert_eq!(tree, flat);
    assert_eq!(tree.cmp(&flat), std::cmp::Ordering::Equal);
    assert_eq!(hash(&tree), hash(&flat));
    tree[0] = 100;
    assert_eq!(flat[0], 0);
    assert!(tree > flat);
}
