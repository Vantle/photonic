use crate::head::Delta;
use crate::input::{Output, Sample};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Loss {
    pub policy: f64,
    pub value: f64,
    pub entropy: f64,
    pub count: usize,
    pub judge: f64,
    pub miss: f64,
    pub judged: usize,
}

impl Loss {
    pub fn merge(self, other: Self) -> Self {
        Self {
            policy: self.policy + other.policy,
            value: self.value + other.value,
            entropy: self.entropy + other.entropy,
            count: self.count + other.count,
            judge: self.judge + other.judge,
            miss: self.miss + other.miss,
            judged: self.judged + other.judged,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weight {
    pub value: f32,
    pub judge: f32,
    pub scale: f32,
}

const MEDIAN: f32 = 0.5;
const UPPER: f32 = 0.98;

fn softmax(logit: &[f32]) -> Vec<f32> {
    let maximum = logit.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exponent = logit
        .iter()
        .map(|value| (value - maximum).exp())
        .collect::<Vec<_>>();
    let total = exponent.iter().sum::<f32>();
    exponent.into_iter().map(|value| value / total).collect()
}

fn policy(output: &Output, sample: &Sample, weight: Weight, loss: &mut Loss) -> Vec<f32> {
    if sample.policy.is_empty() {
        return vec![0.0; output.logit.len()];
    }
    loss.count += 1;
    let probability = softmax(&output.logit);
    let total = sample.policy.iter().sum::<f32>().max(f32::MIN_POSITIVE);
    let mut delta = Vec::with_capacity(probability.len());
    for (&probability, &target) in probability.iter().zip(&sample.policy) {
        let target = target / total;
        if target > 0.0 {
            loss.policy -= f64::from(target * probability.max(1e-12).ln());
        }
        if probability > 0.0 {
            loss.entropy -= f64::from(probability * probability.ln());
        }
        delta.push((probability - target) * weight.scale);
    }
    delta
}

fn pinball(target: f32, estimate: f32, level: f32, factor: f32, loss: &mut Loss) -> f32 {
    let error = target - estimate;
    loss.judge += f64::from((level * error).max((level - 1.0) * error));
    if error > 0.0 {
        -level * factor
    } else {
        (1.0 - level) * factor
    }
}

fn judge(output: &Output, sample: &Sample, weight: Weight, loss: &mut Loss) -> Vec<f32> {
    let (Some(target), [median, upper]) = (sample.judge, output.judge.as_slice()) else {
        return vec![0.0; output.judge.len()];
    };
    loss.miss += f64::from((target - median).abs());
    loss.judged += 1;
    let factor = weight.judge * weight.scale;
    vec![
        pinball(target, *median, MEDIAN, factor, loss),
        pinball(target, *upper, UPPER, factor, loss),
    ]
}

pub fn measure(output: &[Output], sample: &[&Sample], weight: Weight) -> (Loss, Delta) {
    let mut loss = Loss::default();
    let mut change = Delta::default();
    for (sample, output) in sample.iter().zip(output) {
        change.logit.push(policy(output, sample, weight, &mut loss));
        let error = sample.value.map_or(0.0, |target| output.value - target);
        loss.value += f64::from(0.5 * error * error);
        change.value.push(error * weight.value * weight.scale);
        change.judge.push(judge(output, sample, weight, &mut loss));
    }
    (loss, change)
}
