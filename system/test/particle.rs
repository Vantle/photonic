use crate::particle::Particle;

#[test]
fn multiplicity() {
    let particle = Particle::new(["B", "A", "A"]);
    assert_eq!(particle, Particle::new(["A", "B", "A"]));
    assert_eq!(
        particle.remainder(&Particle::new(["A"])),
        Some(Particle::new(["A", "B"]))
    );
    assert_eq!(particle.remainder(&Particle::new(["A", "A", "A"])), None);
    assert_eq!(particle.remainder(&Particle::new(["C"])), None);
    assert_eq!(
        particle.remainder(&Particle::new([])),
        Some(particle.clone())
    );
    assert_eq!(particle.remainder(&particle), Some(Particle::new([])));
}

#[test]
fn algebra() {
    for left in 0usize..64 {
        for right in 0usize..64 {
            let count = |code: usize, value: usize| (code >> (value * 2)) & 3;
            let expand = |code: usize| {
                (0..3).flat_map(move |value| std::iter::repeat_n(value, count(code, value)))
            };
            let particle = Particle::new(expand(left));
            let pattern = Particle::new(expand(right));
            let expected = (0..3)
                .all(|value| count(left, value) >= count(right, value))
                .then(|| {
                    Particle::new((0..3).flat_map(|value| {
                        std::iter::repeat_n(value, count(left, value) - count(right, value))
                    }))
                });
            assert_eq!(particle.remainder(&pattern), expected);
            assert_eq!(
                particle.merge(&pattern),
                Particle::new(expand(left).chain(expand(right)))
            );
        }
    }
}
