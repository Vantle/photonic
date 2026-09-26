use super::Generator;

#[test]
fn deterministic() {
    let mut left = Generator::new(7);
    let mut right = Generator::new(7);
    let mut other = Generator::new(8);
    let sequence = (0..64).map(|_| left.integer()).collect::<Vec<_>>();
    assert_eq!(
        sequence,
        (0..64).map(|_| right.integer()).collect::<Vec<_>>()
    );
    assert_ne!(
        sequence,
        (0..64).map(|_| other.integer()).collect::<Vec<_>>()
    );
}

#[test]
fn golden() {
    let mut generator = Generator::new(7);
    assert_eq!(
        (0..4).map(|_| generator.integer()).collect::<Vec<_>>(),
        [
            0x0e2c_1a00_2aae_913d,
            0x2c0f_c8dd_fa4e_9e14,
            0xb7b3_11b3_b0d4_5872,
            0x6d5d_9f6a_6318_013c,
        ]
    );
    let mut generator = Generator::new(7);
    assert_eq!(
        (0..8).map(|_| generator.below(100)).collect::<Vec<_>>(),
        [5, 17, 71, 42, 96, 46, 72, 32]
    );
}

#[test]
fn bounded() {
    let mut generator = Generator::new(1);
    let mut seen = [0usize; 5];
    for _ in 0..10_000 {
        seen[generator.below(5)] += 1;
        let value = generator.uniform();
        assert!((0.0..1.0).contains(&value));
    }
    assert!(seen.iter().all(|count| (1_700..2_300).contains(count)));
}

#[test]
fn moment() {
    let mut generator = Generator::new(3);
    let sample = (0..100_000).map(|_| generator.normal()).collect::<Vec<_>>();
    let mean = sample.iter().sum::<f64>() / sample.len() as f64;
    let variance = sample
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / sample.len() as f64;
    assert!(mean.abs() < 0.02);
    assert!((variance - 1.0).abs() < 0.03);
    let gumbel = (0..100_000).map(|_| generator.gumbel()).sum::<f64>() / 100_000.0;
    assert!((gumbel - 0.577_215_664_9).abs() < 0.02);
}

#[test]
fn permutation() {
    let mut generator = Generator::new(5);
    let mut value = (0..32).collect::<Vec<_>>();
    generator.shuffle(&mut value);
    let mut sorted = value.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, (0..32).collect::<Vec<_>>());
    assert_ne!(value, sorted);
}
