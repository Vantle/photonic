use super::Ordering;

fn permutation(value: &mut [usize], index: usize, output: &mut Vec<Vec<usize>>) {
    if index == value.len() {
        output.push(value.to_vec());
        return;
    }
    for cursor in index..value.len() {
        value.swap(index, cursor);
        permutation(value, index + 1, output);
        value.swap(index, cursor);
    }
}

#[test]
fn exhaustive() {
    for count in 0..=5 {
        let mut expected = Vec::new();
        permutation(&mut (0..count).collect::<Vec<_>>(), 0, &mut expected);
        expected.sort();
        assert_eq!(Ordering::new(0..count, |_| 0).collect::<Vec<_>>(), expected);
        for encoding in 0..3usize.pow(count as u32) {
            let class = (0..count)
                .map(|index| encoding / 3usize.pow(index as u32) % 3)
                .collect::<Vec<_>>();
            for partition in [false, true] {
                let key = |index: usize| if partition { index % 2 } else { 0 };
                let expected = expected
                    .iter()
                    .filter(|order| {
                        order.windows(2).all(|pair| key(pair[0]) <= key(pair[1]))
                            && (0..count).all(|left| {
                                ((left + 1)..count).all(|right| {
                                    key(order[left]) != key(order[right])
                                        || class[order[left]] != class[order[right]]
                                        || order[left] < order[right]
                                })
                            })
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let actual =
                    Ordering::quotient(0..count, key, |index| class[index]).collect::<Vec<_>>();
                assert_eq!(actual, expected, "{class:?}, partition {partition}");
            }
        }
    }
}
