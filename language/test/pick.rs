use super::Pick;
use std::collections::HashSet;

// The picks come in the order the full enumeration, the first input fastest, first meets each
// choice of options up to reordering the inputs that share their options, each once, and count
// says how many there are.
#[test]
fn enumeration() {
    for (width, class) in [
        (vec![3, 3], vec![0, 0]),
        (vec![3, 2, 3], vec![0, 1, 0]),
        (vec![4, 4, 4], vec![0, 0, 0]),
        (vec![2, 3, 2, 3], vec![0, 1, 0, 1]),
        (vec![1, 5], vec![0, 1]),
        (vec![5, 5, 2, 5], vec![0, 0, 1, 0]),
        (vec![3], vec![0]),
    ] {
        let pick = Pick::new(width.iter().copied().collect(), |input, later| {
            class[input] == class[later]
        });
        let key = |digit: &[usize]| {
            let mut key = (0..width.len())
                .map(|input| (class[input], digit[input]))
                .collect::<Vec<_>>();
            key.sort_unstable();
            key
        };
        let mut seen = HashSet::new();
        let mut expected = Vec::new();
        let mut digit = vec![0; width.len()];
        loop {
            if seen.insert(key(&digit)) {
                expected.push(digit.clone());
            }
            let Some(input) = (0..digit.len()).find(|&input| digit[input] + 1 < width[input])
            else {
                break;
            };
            digit[input] += 1;
            digit[..input].fill(0);
        }
        let mut found = Vec::new();
        let mut digit = pick.first();
        loop {
            found.push(digit.to_vec());
            if !pick.advance(&mut digit) {
                break;
            }
        }
        assert_eq!(found, expected, "{width:?} {class:?}");
        assert_eq!(pick.count(), expected.len(), "{width:?} {class:?}");
    }
}
