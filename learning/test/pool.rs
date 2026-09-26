use crate::corpus::{addition, curated};
use crate::encoding::ATOM;
use crate::import::{Source, define, import};
use crate::objective::Setting;
use crate::pool::{admit, grow};
use crate::problem::Failure;
use crate::task::Task;
use translation::vocabulary::Vocabulary;

fn source(text: &str) -> Source {
    Source {
        origin: "memory".to_owned(),
        text: text.to_owned(),
    }
}

#[test]
fn admission() {
    let setting = Setting::default();
    let task = addition(2);
    let pool = admit(Vec::new(), task.clone(), &setting).unwrap();
    let pool = admit(pool, task.clone(), &setting).unwrap();
    assert_eq!(pool.len(), 1);
    let crowded = Task {
        vocabulary: Vocabulary::alphabet(ATOM).unwrap(),
        ..task.clone()
    };
    assert!(matches!(
        admit(pool.clone(), crowded, &setting),
        Err(Failure::Vocabulary { .. })
    ));
    let full = Task {
        vocabulary: Vocabulary::alphabet(1 << u16::BITS).unwrap(),
        ..task
    };
    assert!(matches!(
        admit(pool, full, &setting),
        Err(Failure::Hidden { .. })
    ));
}

#[test]
fn trainable() {
    let setting = Setting::default();
    let wide = vec!["P"; 40].join(", ");
    let imported = import(
        "go",
        &[source(&format!("Go, [Go] ({wide})"))],
        &[],
        &setting.thorough(),
    )
    .unwrap();
    assert_eq!(
        admit(Vec::new(), imported, &setting),
        Err(Failure::Limit {
            task: "go".to_owned()
        })
    );
    let tested = define(
        "spread",
        &[(source("A"), source("B")), (source("Go"), source(&wide))],
    )
    .unwrap();
    assert_eq!(
        admit(Vec::new(), tested, &setting),
        Err(Failure::Output {
            task: "spread".to_owned(),
            test: 1
        })
    );
    let narrow = define("rename", &[(source("A"), source("B"))]).unwrap();
    assert_eq!(admit(Vec::new(), narrow, &setting).unwrap().len(), 1);
}

#[test]
fn fresh() {
    let setting = Setting::default();
    let pool = curated();
    let length = pool.len() as u64;
    let first = grow(pool, 1, 7, &setting).unwrap();
    let second = grow(first.clone(), 1, 7 ^ length ^ (length + 1), &setting).unwrap();
    assert_ne!(
        first.last().map(|task| &task.example),
        second.last().map(|task| &task.example)
    );
}
