use crate::curriculum::{Curriculum, focus, grade, prefix};
use crate::objective::Setting;
use crate::solution::Failure;
use crate::task::{Goal, Task};
use std::time::Duration;

#[test]
fn breed() {
    let setting = Setting::default();
    let mut curriculum = Curriculum::default();
    let first = curriculum.breed(1, 7, &setting).unwrap();
    let second = curriculum.breed(2, 7, &setting).unwrap();
    assert_eq!(first.name, format!("{}0", prefix(1)));
    assert_eq!(second.name, format!("{}1", prefix(2)));
    assert!(first.reference.is_none() && second.reference.is_none());
    assert_eq!(curriculum.bred, 2);
    let exam = grade(first.clone(), &setting, Duration::from_secs(30))
        .unwrap()
        .unwrap();
    assert!(exam.cost.is_finite() && exam.size > 0);
    assert_eq!(exam.task.goal, Some(setting.goal));
    let unbounded = Setting {
        goal: Goal::new(setting.goal.processor(), 0.0).unwrap(),
        ..setting
    };
    assert_eq!(
        grade(first, &unbounded, Duration::from_secs(1)),
        Err(Failure::Unbounded)
    );
}

#[test]
fn rehearse() {
    let named = |name: &str| Task {
        name: name.to_owned(),
        vocabulary: translation::vocabulary::Vocabulary::default(),
        example: Vec::new(),
        holdout: Vec::new(),
        reference: None,
        goal: None,
    };
    let pool = [
        "level1.0",
        "level1.1",
        "level1.2",
        "level2.3",
        "level2.4",
        "level2.5",
        "level10.6",
    ]
    .into_iter()
    .map(named)
    .collect::<Vec<_>>();
    let (count, chosen) = focus(&pool, 2, 0.25, 3);
    assert_eq!(count, 3);
    assert_eq!(chosen.len(), 4);
    assert!(chosen[..3].iter().all(|name| name.starts_with(&prefix(2))));
    assert!(chosen[3].starts_with(&prefix(1)));
    let (count, chosen) = focus(&pool, 1, 0.25, 3);
    assert_eq!((count, chosen.len()), (3, 3));
}

#[test]
fn level() {
    let text = |level: usize| format!(r#"{{"level":{level},"bred":0,"exam":[],"mark":[]}}"#);
    assert!(serde_json::from_str::<Curriculum>(&text(0)).is_err());
    let state = serde_json::from_str::<Curriculum>(&text(2)).unwrap();
    assert_eq!(state.level, 2);
}
