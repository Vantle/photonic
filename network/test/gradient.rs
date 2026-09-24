use super::support::{configuration, sample};
use crate::input::Sample;
use crate::loss::Weight;
use crate::model::Model;
use random::Generator;

const WEIGHT: Weight = Weight {
    value: 0.7,
    judge: 0.5,
    scale: 1.0,
};

fn total(model: &Model, sample: &[&Sample]) -> f64 {
    let mut scratch = vec![0.0; model.size()];
    let loss = model.gradient(sample, &mut scratch, WEIGHT);
    loss.policy + f64::from(WEIGHT.value) * loss.value + f64::from(WEIGHT.judge) * loss.judge
}

#[test]
fn finite() {
    let mut generator = Generator::new(31);
    let configuration = configuration();
    let mut model = Model::new(configuration.clone(), &mut generator);
    for value in &mut model.parameter {
        *value += generator.normal() as f32 * 0.3;
    }
    let sample = (0..3)
        .map(|_| sample(&mut generator, &configuration))
        .collect::<Vec<_>>();
    let reference = sample.iter().collect::<Vec<_>>();
    let mut analytic = vec![0.0; model.size()];
    model.gradient(&reference, &mut analytic, WEIGHT);
    let step = 2e-3f32;
    let mut failure = Vec::new();
    for (index, &exact) in analytic.iter().enumerate() {
        let original = model.parameter[index];
        model.parameter[index] = original + step;
        let above = total(&model, &reference);
        model.parameter[index] = original - step;
        let below = total(&model, &reference);
        model.parameter[index] = original;
        let numeric = (above - below) / (2.0 * f64::from(step));
        let exact = f64::from(exact);
        let tolerance = 2e-2 * numeric.abs().max(exact.abs()) + 2e-3;
        if (numeric - exact).abs() > tolerance {
            failure.push((index, exact, numeric));
        }
    }
    assert!(
        failure.is_empty(),
        "{} of {} gradients disagree: {:?}",
        failure.len(),
        model.size(),
        &failure[..failure.len().min(12)]
    );
}

#[test]
fn separable() {
    let mut generator = Generator::new(32);
    let configuration = configuration();
    let model = Model::new(configuration.clone(), &mut generator);
    let sample = (0..4)
        .map(|_| sample(&mut generator, &configuration))
        .collect::<Vec<_>>();
    let mut together = vec![0.0; model.size()];
    let joint = model.gradient(&sample.iter().collect::<Vec<_>>(), &mut together, WEIGHT);
    let mut apart = vec![0.0; model.size()];
    let mut policy = 0.0;
    for sample in &sample {
        policy += model.gradient(&[sample], &mut apart, WEIGHT).policy;
    }
    assert!((joint.policy - policy).abs() < 1e-4);
    for (left, right) in together.iter().zip(&apart) {
        assert!((left - right).abs() <= 1e-5 + 1e-4 * left.abs().max(right.abs()));
    }
}
