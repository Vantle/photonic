use super::Sample;

#[test]
fn median() {
    let mut sample = Sample::new(3.0);
    assert_eq!(sample.spread().median, 3.0);
    sample.push(1.0);
    let even = sample.spread();
    assert_eq!(
        (even.count, even.minimum, even.median, even.maximum),
        (2, 1.0, 2.0, 3.0)
    );
    sample.push(10.0);
    assert_eq!(sample.spread().median, 3.0);
    sample.push(4.0);
    assert_eq!(sample.spread().median, 3.5);
}
