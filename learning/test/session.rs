use crate::corpus::{addition, boolean};
use crate::encoding::Shape;
use crate::home::{self, Home};
use crate::pool::prepare;
use crate::session::{Event, Origin, Setting, run};
use crate::{play, search, train};
use std::time::Duration;

#[test]
fn session() {
    let path = std::env::temp_dir().join(format!("learning-session-{}", std::process::id()));
    let home = Home::open(path.clone()).unwrap();
    let pool = vec![addition(2).conceal(2), boolean().remove(0).conceal(2)];
    let objective = crate::objective::Setting::default();
    let (problem, failure) = prepare(&pool, &objective);
    assert!(failure.is_empty());
    let setting = Setting {
        placement: crate::session::Placement::Automatic,
        worker: Some(2),
        trainer: 1,
        frozen: false,
        duration: Some(Duration::from_secs(4)),
        report: Duration::from_secs(1),
        seed: 3,
        shape: Shape {
            width: 16,
            depth: 1,
            head: 2,
            hidden: 32,
            key: 8,
        },
        capacity: 10_000,
        teach: Some(Duration::from_secs(1)),
        play: play::Setting {
            game: 2,
            race: 1,
            search: search::Setting {
                simulation: 4,
                considered: 4,
                step: 6,
                ..search::Setting::default()
            },
            ..play::Setting::default()
        },
        train: train::Setting {
            batch: 8,
            chunk: 4,
            minimum: 16,
            ..train::Setting::default()
        },
    };
    let mut report = 0;
    let summary = run(&home, problem.clone(), vec![0], &setting, |event| {
        if let Event::Report(_) = event {
            report += 1;
        }
    })
    .unwrap();
    assert!(report >= 2);
    assert!(summary.step > 0);
    assert!(home.file(home::CHECKPOINT).exists());
    assert!(home.file(home::ARCHIVE).exists());
    let mut restored = false;
    run(
        &home,
        problem,
        Vec::new(),
        &Setting {
            duration: Some(Duration::from_secs(1)),
            frozen: true,
            ..setting
        },
        |event| {
            if let Event::Start {
                origin: Origin::Restored,
                ..
            } = event
            {
                restored = true;
            }
        },
    )
    .unwrap();
    assert!(restored);
    std::fs::remove_dir_all(path).unwrap();
}
