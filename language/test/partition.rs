use crate::link::Link;

const KIND: [Link; 4] = [Link::Context, Link::World, Link::Member, Link::Particle];

#[test]
fn classification() {
    for count in 0..=6 {
        for encoding in 0..4usize.pow(count) {
            let value = (0..count)
                .map(|index| (encoding >> (index * 2)) & 3)
                .collect::<Vec<_>>();
            let unique = value
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>();
            let expected = value
                .iter()
                .map(|value| {
                    unique
                        .iter()
                        .position(|candidate| candidate == value)
                        .unwrap()
                })
                .collect::<Vec<_>>();
            assert_eq!(crate::partition::classify(&value), expected);
        }
    }
}

#[test]
fn graph() {
    for encoding in 0..512usize {
        let edge = (0..3)
            .map(|source| {
                (0..3)
                    .filter(|&target| encoding & (1 << (source * 3 + target)) != 0)
                    .map(|target| (KIND[source % 2], target))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for label in 0..8 {
            let color = (0..3).map(|index| (label >> index) & 1).collect::<Vec<_>>();
            verify(&edge, color);
        }
    }
}

fn verify(edge: &[Vec<(Link, usize)>], color: Vec<usize>) {
    let mut expected = color.clone();
    loop {
        let signature = edge
            .iter()
            .enumerate()
            .map(|(index, edge)| {
                let mut adjacent = edge
                    .iter()
                    .map(|&(kind, target)| (kind, expected[target]))
                    .collect::<Vec<_>>();
                adjacent.sort();
                (expected[index], adjacent)
            })
            .collect::<Vec<_>>();
        let unique = signature.iter().collect::<std::collections::BTreeSet<_>>();
        let next = signature
            .iter()
            .map(|value| {
                unique
                    .iter()
                    .position(|candidate| *candidate == value)
                    .unwrap()
            })
            .collect::<Vec<_>>();
        if next == expected {
            break;
        }
        expected = next;
    }
    let graph = crate::graph::Graph::new(
        edge.len(),
        edge.iter().enumerate().flat_map(|(source, adjacent)| {
            adjacent
                .iter()
                .map(move |&(kind, target)| (source, target, kind))
        }),
    );
    for (index, adjacent) in edge.iter().enumerate() {
        assert_eq!(&graph[index], adjacent);
    }
    assert_eq!(crate::partition::refine(&graph, color), expected);
}

#[test]
fn propagation() {
    verify(&[], Vec::new());
    verify(&[Vec::new()], vec![0]);
    for length in 2..65 {
        let edge = (0..length)
            .map(|index| {
                if index + 1 < length {
                    vec![
                        (KIND[0], index + 1),
                        (KIND[0], index + 1),
                        (KIND[1], index + 1),
                    ]
                } else {
                    Vec::new()
                }
            })
            .collect::<Vec<_>>();
        verify(&edge, vec![0; length]);
    }
    let mut seed = 1u64;
    for length in 4..33 {
        for _ in 0..64 {
            let mut edge = vec![Vec::new(); length];
            for adjacent in &mut edge {
                for target in 0..length {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    if seed >> 60 < 3 {
                        adjacent.push((KIND[(seed >> 32) as usize % 4], target));
                    }
                }
            }
            verify(&edge, (0..length).map(|index| index % 3).collect());
        }
    }
}

#[test]
fn scheduling() {
    for length in [2, 7, 32, 65, 128] {
        let edge = (0..length)
            .map(|index| {
                let next = (index + 1) % length;
                vec![
                    (KIND[0], next),
                    (KIND[0], next),
                    (KIND[1], index),
                    (KIND[2], length - 1),
                ]
            })
            .collect::<Vec<_>>();
        verify(&edge, (0..length).map(|index| usize::MAX - index).collect());
        verify(
            &edge,
            (0..length)
                .map(|index| usize::from(index == 0) * 91)
                .collect(),
        );
        verify(&edge, (0..length).map(|index| (index % 7) * 91).collect());
        let reverse = edge
            .iter()
            .rev()
            .map(|adjacent| {
                adjacent
                    .iter()
                    .rev()
                    .map(|&(kind, target)| (kind, length - 1 - target))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        verify(
            &reverse,
            (0..length)
                .map(|index| usize::from(index + 1 == length) * 91)
                .collect(),
        );
    }
}
