use super::support::{A, B};
use crate::atom::Atom;
use crate::observation::{Observation, Occurrence};
use crate::value::Value;

fn ring(rotation: u32) -> Observation {
    let occurrence = |id, atom| Occurrence {
        id,
        value: Value::Atom(Atom(atom)),
    };
    Observation::new(
        (0..3)
            .map(|index| (index + rotation) % 3)
            .map(|index| {
                vec![
                    occurrence(index, A),
                    occurrence(10 + index, B),
                    occurrence(10 + (index + 1) % 3, B),
                ]
            })
            .collect(),
    )
}

#[test]
fn same() {
    let original = ring(0);
    let rotated = ring(1);
    assert!(original.key(0).is_err());
    assert!(original.same(&original, 0));
    assert!(!original.same(&rotated, 0));
    assert!(original.same(&rotated, 4_096));
}
