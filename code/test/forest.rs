use crate::forest::Forest;

#[test]
fn group() {
    let mut forest = Forest::new(7);
    forest.join(5, 2);
    forest.join(6, 5);
    forest.join(4, 1);
    forest.join(3, 3);
    assert_eq!(forest.root(6), 2);
    assert_eq!(forest.root(4), 1);
    assert_eq!(forest.root(3), 3);
    assert_eq!(
        forest.group(),
        vec![vec![0], vec![1, 4], vec![2, 5, 6], vec![3]]
    );
    forest.join(4, 6);
    assert_eq!(forest.root(6), 1);
    assert_eq!(forest.group(), vec![vec![0], vec![1, 2, 4, 5, 6], vec![3]]);
}
