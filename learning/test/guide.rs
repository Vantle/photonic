use crate::edit::Bound;
use crate::encoding::{DIMENSION, Shape};
use crate::guide::{Effort, Network, search};
use crate::objective::Setting;
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
        lift::program(&photonic::lowering::parse(text).unwrap(), &mut vocabulary)
            .unwrap()
            .1
    };
    let input = parse("A");
    let output = parse("B");
    Task {
        name: "rename".to_owned(),
        vocabulary,
        hidden: 0,
        example: vec![Example {
            input,
            output: Observation::from(&output),
        }],
        holdout: Vec::new(),
        reference: None,
        goal: None,
    }
}

#[test]
fn rename() {
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
        std::env::temp_dir().join(format!("learning-guide-{}.checkpoint", std::process::id()));
    checkpoint::save(&path, &model, &optimizer).unwrap();
    let mut network = Network::load(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
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
    assert!((evaluation.cost - 1.6).abs() < 1e-9);
    assert!(
        guidance
            .first
            .is_some_and(|first| first <= guidance.expanded)
    );
}
