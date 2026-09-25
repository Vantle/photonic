use super::support::{configuration, input, sample};
use crate::engine::Engine;
use crate::failure::Failure;
use network::loss::Weight;
use network::model::Model;
use network::optimizer::{Optimizer, Setting};
use random::Generator;

#[test]
fn agreement() {
    let mut generator = Generator::new(7);
    let mut model = Model::new(configuration(), &mut generator);
    for value in &mut model.parameter {
        *value += generator.normal() as f32 * 0.2;
    }
    let mut engine = match Engine::new(&model) {
        Ok(engine) => engine,
        Err(Failure::Unavailable(_)) => return,
        Err(error) => panic!("{error}"),
    };
    let input = (0..9)
        .map(|_| input(&mut generator, &configuration()))
        .collect::<Vec<_>>();
    let reference = input.iter().collect::<Vec<_>>();
    let expected = model.infer(&reference);
    let actual = engine.infer(&reference).unwrap();
    for (expected, actual) in expected.iter().zip(&actual) {
        assert!(
            (expected.value - actual.value).abs() < 1e-3,
            "{} {}",
            expected.value,
            actual.value
        );
        assert_eq!(expected.logit.len(), actual.logit.len());
        for (left, right) in expected.logit.iter().zip(&actual.logit) {
            assert!((left - right).abs() < 1e-3, "{left} {right}");
        }
    }
    for value in &mut model.parameter {
        *value *= 0.5;
    }
    engine.load(&model.parameter).unwrap();
    assert!(matches!(
        engine.load(&model.parameter[1..]),
        Err(Failure::Length { .. })
    ));
    let expected = model.infer(&reference);
    let actual = engine.infer(&reference).unwrap();
    assert!((expected[0].value - actual[0].value).abs() < 1e-3);
    assert!(engine.infer(&[]).unwrap().is_empty());
}

fn close(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance * left.abs().max(right.abs()).max(1.0)
}

#[test]
fn training() {
    let mut generator = Generator::new(11);
    let configuration = configuration();
    let mut model = Model::new(configuration.clone(), &mut generator);
    for value in &mut model.parameter {
        *value += generator.normal() as f32 * 0.2;
    }
    let mut engine = match Engine::new(&model) {
        Ok(engine) => engine,
        Err(Failure::Unavailable(_)) => return,
        Err(error) => panic!("{error}"),
    };
    let sample = (0..7)
        .map(|_| sample(&mut generator, &configuration))
        .collect::<Vec<_>>();
    let reference = sample.iter().collect::<Vec<_>>();
    let weight = Weight {
        value: 0.5,
        judge: 0.7,
        scale: 1.0 / reference.len() as f32,
    };
    let rate = 1e-3;
    assert!(matches!(
        engine.train(&reference, weight, rate),
        Err(Failure::Unprepared)
    ));
    let mut gradient = vec![0.0; model.size()];
    let expected = model.gradient(&reference, &mut gradient, weight);
    let mut optimizer = Optimizer::new(model.size(), Setting::default());
    let mut parameter = model.parameter.clone();
    let norm = optimizer.update(&mut parameter, &gradient, model.decay(), rate);
    engine
        .prepare(&Optimizer::new(model.size(), Setting::default()))
        .unwrap();
    let (loss, actual) = engine.train(&reference, weight, rate).unwrap();
    assert!(close(loss.policy, expected.policy, 1e-4));
    assert!(close(loss.value, expected.value, 1e-4));
    assert!(
        close(f64::from(actual), f64::from(norm), 1e-3),
        "{actual} {norm}"
    );
    let state = engine.optimizer().unwrap();
    assert_eq!(state.step, 1);
    let largest = gradient
        .iter()
        .fold(0.0f32, |largest, value| largest.max(value.abs()));
    for (left, right) in state.moment.iter().zip(&optimizer.moment) {
        assert!((left - right).abs() <= 1e-4 * largest, "{left} {right}");
    }
    for (left, right) in state.velocity.iter().zip(&optimizer.velocity) {
        assert!(
            (left - right).abs() <= 1e-4 * largest * largest,
            "{left} {right}"
        );
    }
    let updated = engine.parameter();
    for ((left, right), change) in updated.iter().zip(&parameter).zip(&gradient) {
        let bound = if change.abs() > 1e-3 * largest {
            1e-5
        } else {
            2.5 * rate
        };
        assert!((left - right).abs() <= bound, "{left} {right} {change}");
    }
    model.parameter = updated;
    let subset = &reference[..3];
    let mut scratch = vec![0.0; model.size()];
    let expected = model.gradient(subset, &mut scratch, weight);
    let (loss, _) = engine.train(subset, weight, rate).unwrap();
    assert!(close(loss.policy, expected.policy, 1e-4));
    assert!(close(loss.value, expected.value, 1e-4));
    assert_eq!(engine.optimizer().unwrap().step, 2);
    let input = subset
        .iter()
        .map(|sample| &sample.input)
        .collect::<Vec<_>>();
    model.parameter = engine.parameter();
    let expected = model.infer(&input);
    let actual = engine.infer(&input).unwrap();
    for (expected, actual) in expected.iter().zip(&actual) {
        assert!(close(
            f64::from(expected.value),
            f64::from(actual.value),
            1e-3
        ));
    }
}
