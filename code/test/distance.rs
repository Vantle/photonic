use super::support::{A, B, C, configuration};
use crate::distance::distance;

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
    let left = configuration(&[&[A, B], &[C], &[A]]);
    let right = configuration(&[&[B], &[C, C]]);
    assert!((distance(&left, &right) - distance(&right, &left)).abs() < 1e-12);
}
