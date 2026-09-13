use crate::particle::Particle;
use crate::rule::Rule;

#[test]
fn broadcast() {
    let rule = Rule {
        input: vec![
            Particle::new(["A"]),
            Particle::new(["B"]),
            Particle::new(["C"]),
        ],
        output: vec![Particle::new(["D"]), Particle::new(["E"])],
    };
    let binding = [
        Particle::new(["A", "X"]),
        Particle::new(["B", "Y"]),
        Particle::new(["C", "Z"]),
    ];
    assert_eq!(
        rule.apply(&binding),
        Some(vec![
            Particle::new(["D", "X", "Y", "Z"]),
            Particle::new(["E", "X", "Y", "Z"])
        ])
    );
    assert_eq!(rule.apply(&binding[..2]), None);
    assert_eq!(
        rule.apply(&[binding[1].clone(), binding[0].clone(), binding[2].clone()]),
        None
    );
}

#[test]
fn duplicate() {
    let rule = Rule {
        input: vec![Particle::new(["A"])],
        output: vec![Particle::new(["B"]), Particle::new(["B"])],
    };
    assert_eq!(
        rule.apply(&[Particle::new(["A", "A", "C"])]),
        Some(vec![Particle::new(["A", "B", "C"]); 2])
    );
    assert_eq!(rule.apply(&[Particle::new(["C"])]), None);
}

#[test]
fn boundary() {
    let rule = Rule {
        input: vec![],
        output: vec![Particle::new(["A"])],
    };
    assert_eq!(rule.apply(&[]), Some(vec![Particle::new(["A"])]));
    let rule = Rule {
        input: vec![Particle::new(["A"])],
        output: vec![],
    };
    assert_eq!(rule.apply(&[Particle::new(["A", "B"])]), Some(vec![]));
}

#[test]
fn amplification() {
    let divergence = Rule {
        input: vec![Particle::new(["A"])],
        output: vec![Particle::new(["B"]), Particle::new(["C"])],
    };
    let decoherence = Rule {
        input: vec![Particle::new(["B"]), Particle::new(["C"])],
        output: vec![Particle::new(["A"])],
    };
    let branch = divergence.apply(&[Particle::new(["A", "X"])]).unwrap();
    assert_eq!(
        decoherence.apply(&branch),
        Some(vec![Particle::new(["A", "X", "X"])])
    );
}

#[test]
fn nested() {
    let original = Rule {
        input: vec![Particle::new(["A"])],
        output: vec![Particle::new(["B"])],
    };
    let replacement = Rule {
        input: vec![Particle::new(["A"])],
        output: vec![Particle::new(["C"])],
    };
    let unrelated = Rule {
        input: vec![Particle::new(["X"])],
        output: vec![Particle::new(["Y"])],
    };
    let rule = Rule {
        input: vec![Particle::new([original.clone()])],
        output: vec![Particle::new([replacement.clone()])],
    };
    let binding = Particle::new([original.clone(), unrelated.clone()]);
    assert_eq!(
        rule.apply(std::slice::from_ref(&binding)),
        Some(vec![Particle::new([
            replacement.clone(),
            unrelated.clone()
        ])])
    );
    assert_eq!(binding, Particle::new([original.clone(), unrelated]));
    assert_eq!(rule.apply(&[Particle::new([replacement.clone()])]), None);
    assert_eq!(
        rule.apply(&[Particle::new([original.clone(), original.clone()])]),
        Some(vec![Particle::new([original, replacement])])
    );
    let outer = Rule {
        input: vec![Particle::new([rule.clone()])],
        output: vec![],
    };
    assert_eq!(outer.apply(&[Particle::new([rule])]), Some(vec![]));
}
