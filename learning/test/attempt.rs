use crate::archive::{Archive, Coverage};
use crate::attempt::{Event, Setting, run};
use crate::edit::Bound;
use crate::home::{self, Home};
use crate::import::{Source, define};
use crate::objective;
use crate::solution::Budget;
use std::time::Duration;

fn source(text: &str) -> Source {
    Source {
        origin: "memory".to_owned(),
        text: text.to_owned(),
    }
}

#[test]
fn coverage() {
    let path = std::env::temp_dir().join(format!("learning-attempt-{}", std::process::id()));
    let home = Home::open(path.clone()).unwrap();
    let pool = vec![define("rename", &[(source("A"), source("B"))]).unwrap()];
    let chosen = ["rename".to_owned()];
    for (depth, expected) in [(1, Coverage::Flat), (0, Coverage::Whole)] {
        let setting = Setting {
            objective: objective::Setting::default(),
            bound: Bound {
                depth,
                ..Bound::default()
            },
            budget: Budget::default(),
            guide: Duration::ZERO,
            blind: true,
            deadline: None,
        };
        let mut seen = Vec::new();
        let count = run(&home, &pool, &chosen, &setting, |event| {
            if let Event::Exhaustion { coverage, .. } = event {
                seen.push(coverage);
            }
        })
        .unwrap();
        assert_eq!(seen, vec![expected]);
        assert_eq!(count.total, 1);
        assert_eq!(
            (count.proven, count.flat),
            if expected == Coverage::Whole {
                (1, 0)
            } else {
                (0, 1)
            }
        );
        let archive: Archive = home.load(home::ARCHIVE).unwrap().unwrap();
        assert_eq!(
            archive
                .best("rename")
                .and_then(|record| record.proof)
                .map(|proof| proof.coverage),
            Some(expected)
        );
    }
    assert!(home.file("program/rename.wave").exists());
    std::fs::remove_dir_all(path).unwrap();
}
