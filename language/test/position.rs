use crate::position::Index;

#[test]
fn mutation() {
    let mut index = Index::default();
    let mut expected = Vec::new();
    let mut position = Vec::new();
    let mut seed = 41u64;
    for step in 0..8192 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        if expected.is_empty() || step % 3 != 0 {
            let value = position.len();
            position.push(index.insert(value));
            expected.push(value);
        } else {
            let rank = (seed >> 32) as usize % expected.len();
            let value = expected.remove(rank);
            index.remove(position[value]);
        }
        if let Some(&value) = expected.first() {
            assert_eq!(index.select(0), value);
            assert_eq!(index.rank(position[value]), 0);
        }
        if let Some(&value) = expected.last() {
            assert_eq!(index.select(expected.len() - 1), value);
            assert_eq!(index.rank(position[value]), expected.len() - 1);
        }
        index.compact(&mut position);
        for (rank, &value) in expected.iter().enumerate() {
            assert_eq!(index.select(rank), value);
            assert_eq!(index.rank(position[value]), rank);
        }
    }
    for value in expected.into_iter().rev() {
        index.remove(position[value]);
        index.compact(&mut position);
    }
    position.push(index.insert(position.len()));
    assert_eq!(index.select(0), position.len() - 1);
    assert_eq!(index.rank(*position.last().unwrap()), 0);
    assert!(index.retained() < 140);
}
