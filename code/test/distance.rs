use super::support::{A, B, C, configuration};
use crate::configuration::Configuration;
use crate::distance::distance;

fn padded(prefix: &[&[u16]], first: u16) -> Configuration {
    let single = (first..first + 9).map(|atom| [atom]).collect::<Vec<_>>();
    configuration(
        &prefix
            .iter()
            .copied()
            .chain(single.iter().map(|atom| &atom[..]))
            .collect::<Vec<_>>(),
    )
}

#[test]
fn identical() {
    let value = configuration(&[&[A, B], &[C]]);
    assert!(distance(&value, &value).abs() < f64::EPSILON);
    assert!(distance(&configuration(&[]), &configuration(&[])).abs() < f64::EPSILON);
}

#[test]
fn disjoint() {
    let value = distance(&configuration(&[&[A]]), &configuration(&[]));
    assert!((value - 1.0).abs() < f64::EPSILON);
}

#[test]
fn partial() {
    let near = distance(
        &configuration(&[&[A, B], &[C]]),
        &configuration(&[&[A], &[C]]),
    );
    let far = distance(
        &configuration(&[&[A, B], &[C]]),
        &configuration(&[&[B], &[A]]),
    );
    assert!(near > 0.0);
    assert!(near < far);
    assert!(far <= 1.0);
}

#[test]
fn symmetric() {
    let pair = [
        (
            configuration(&[&[A, B], &[C], &[A]]),
            configuration(&[&[B], &[C, C]]),
        ),
        (
            configuration(&[&[A], &[A, B]]),
            padded(&[&[A, B], &[A, C]], 10),
        ),
        (
            padded(&[&[A], &[A, B]], 20),
            padded(&[&[A, B], &[A, C]], 10),
        ),
    ];
    for (left, right) in &pair {
        assert!((distance(left, right) - distance(right, left)).abs() < 1e-12);
    }
    assert!((distance(&pair[1].0, &pair[1].1) - 19.0 / 29.0).abs() < 1e-12);
}
