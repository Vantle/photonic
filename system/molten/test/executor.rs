use molten::executor::{Executor, Failure};

#[test]
fn order() {
    let suffix = String::from("done");
    for worker in [1, 2, 4] {
        let executor = Executor::new(worker).unwrap();
        let input = (0..257).map(|index| index.to_string()).collect::<Vec<_>>();
        let expected = input
            .iter()
            .map(|value| format!("{value}:{suffix}"))
            .collect::<Vec<_>>();
        assert_eq!(
            executor.map(input, |value| format!("{value}:{suffix}")),
            expected
        );
        assert!(executor.map(Vec::<String>::new(), |value| value).is_empty());
    }
}

#[test]
fn zero() {
    assert!(matches!(Executor::new(0), Err(Failure::Zero)));
}
