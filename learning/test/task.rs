use crate::corpus::addition;
use crate::task::{Failure, Goal, Task, find};
use translation::vocabulary::Vocabulary;

#[test]
fn conceal() {
    let task = addition(2);
    let count = task.vocabulary.len();
    assert_eq!(task.clone().conceal(2).unwrap().vocabulary.len(), count + 2);
    let full = Task {
        vocabulary: Vocabulary::alphabet(1 << u16::BITS).unwrap(),
        ..task
    };
    assert!(matches!(
        full.conceal(1),
        Err(translation::failure::Failure::Vocabulary { .. })
    ));
}

#[test]
fn goal() {
    assert_eq!(Goal::new(0.0, 0.05), Err(Failure::Processor));
    assert_eq!(Goal::new(f64::INFINITY, 0.05), Err(Failure::Processor));
    assert_eq!(Goal::new(4.0, -0.5), Err(Failure::Size));
    assert_eq!(Goal::new(4.0, f64::NAN), Err(Failure::Size));
    let goal = Goal::new(2.0, 0.0).unwrap();
    let text = serde_json::to_string(&goal).unwrap();
    assert_eq!(text, r#"{"processor":2.0,"size":0.0}"#);
    assert_eq!(serde_json::from_str::<Goal>(&text).unwrap(), goal);
    for text in [
        r#"{"processor":0.0,"size":0.05}"#,
        r#"{"processor":4.0,"size":-1.0}"#,
        r#"{"processor":4.0,"size":0.05,"extra":1}"#,
    ] {
        assert!(serde_json::from_str::<Goal>(text).is_err(), "{text}");
    }
}

#[test]
fn name() {
    let task = Task {
        name: "../escape".to_owned(),
        ..addition(2)
    };
    let text = serde_json::to_string(&task).unwrap();
    let failure = serde_json::from_str::<Task>(&text).unwrap_err();
    assert!(
        failure.to_string().contains("cannot name a task"),
        "{failure}"
    );
    let pool = [addition(2)];
    assert_eq!(find(&pool, "addition.2").unwrap().name, "addition.2");
    assert_eq!(
        find(&pool, "missing").unwrap_err(),
        Failure::Missing {
            name: "missing".to_owned()
        }
    );
}
