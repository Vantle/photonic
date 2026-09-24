use crate::curriculum::{Curriculum, focus, grade, prefix};
use crate::objective::Setting;
use crate::task::Task;
use std::time::Duration;

#[test]
fn breed() {
    let setting = Setting::default();
    let mut curriculum = Curriculum::default();
    let first = curriculum.breed(1, 7, &setting);
    let second = curriculum.breed(2, 7, &setting);
    assert_eq!(first.name, format!("{}0", prefix(1)));
    assert_eq!(second.name, format!("{}1", prefix(2)));
    assert!(first.reference.is_none() && second.reference.is_none());
    assert_eq!(first.hidden, 4);
    assert_eq!(curriculum.bred, 2);
    let exam = grade(first, &setting, Duration::from_secs(30)).unwrap();
    assert!(exam.cost.is_finite() && exam.size > 0);
}

#[test]
fn rehearse() {
    let named = |name: &str| Task {
        name: name.to_owned(),
        vocabulary: translation::vocabulary::Vocabulary::default(),
        hidden: 0,
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
