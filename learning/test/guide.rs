use crate::edit::Bound;
use crate::encoding::{DIMENSION, Shape};
use crate::guide::{Effort, Network, search};
use crate::objective::{Setting, TOLERANCE};
use crate::task::{Example, Task};
use code::observation::Observation;
use network::checkpoint;
use network::model::Model;
use network::optimizer::Optimizer;
use random::Generator;
use std::time::Duration;
use translation::lift;
use translation::vocabulary::Vocabulary;

fn task() -> Task {
    let mut vocabulary = Vocabulary::default();
    let mut parse = |text: &str| {
        lift::program(&frontend::lowering::parse(text).unwrap(), &mut vocabulary)
            .unwrap()
            .1
    };
    let input = parse("A");
    let output = parse("B");
    Task {
        name: "rename".to_owned(),
        vocabulary,
        example: vec![Example {
            input,
            output: Observation::from(&output),
        }],
        holdout: Vec::new(),
        reference: None,
        goal: None,
    }
}

fn untrained(name: &str) -> Network {
    let shape = Shape {
        width: DIMENSION,
        depth: 1,
        head: 1,
        hidden: 32,
        key: DIMENSION,
    };
    let model = Model::new(shape.architecture(), &mut Generator::new(1));
    let optimizer = Optimizer::new(model.size(), network::optimizer::Setting::default());
    let path =
        std::env::temp_dir().join(format!("learning-{name}-{}.checkpoint", std::process::id()));
    checkpoint::save(&path, &model, &optimizer).unwrap();
    let network = Network::load(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    network
}

#[test]
fn rename() {
    let mut network = untrained("rename");
    let guidance = search(
        &task(),
        &Bound::default(),
        &Setting::default(),
        Effort {
            time: Duration::from_secs(60),
            expansion: 100_000,
        },
        1.6,
        &mut network,
    );
    let (_, evaluation) = guidance.best.unwrap();
    assert!((evaluation.cost - 1.6).abs() < TOLERANCE);
    assert!(
        guidance
            .first
            .is_some_and(|first| first <= guidance.expanded)
    );
}

#[test]
fn limit() {
    let mut network = untrained("limit");
    let guidance = search(
        &task(),
        &Bound::default(),
        &Setting::default(),
        Effort {
            time: Duration::from_secs(60),
            expansion: 10,
        },
        0.0,
        &mut network,
    );
    assert_eq!(guidance.expanded, 10);
}
