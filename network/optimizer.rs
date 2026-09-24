use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Setting {
    pub rate: f32,
    pub decay: f32,
    pub first: f32,
    pub second: f32,
    pub epsilon: f32,
    pub clip: f32,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            rate: 3e-4,
            decay: 1e-4,
            first: 0.9,
            second: 0.95,
            epsilon: 1e-8,
            clip: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Optimizer {
    pub setting: Setting,
    pub moment: Vec<f32>,
    pub velocity: Vec<f32>,
    pub step: u64,
}

impl Optimizer {
    pub fn new(size: usize, setting: Setting) -> Self {
        Self {
            setting,
            moment: vec![0.0; size],
            velocity: vec![0.0; size],
            step: 0,
        }
    }

    pub fn update(
        &mut self,
        parameter: &mut [f32],
        gradient: &[f32],
        decay: &[Range<usize>],
        rate: f32,
    ) -> f32 {
        let norm = gradient
            .iter()
            .map(|value| f64::from(*value) * f64::from(*value))
            .sum::<f64>()
            .sqrt() as f32;
        let clip = if norm > self.setting.clip && norm > 0.0 {
            self.setting.clip / norm
        } else {
            1.0
        };
        self.step += 1;
        let step = self.step as i32;
        let first = 1.0 - self.setting.first.powi(step);
        let second = 1.0 - self.setting.second.powi(step);
        for range in decay {
            for value in &mut parameter[range.clone()] {
                *value -= rate * self.setting.decay * *value;
            }
        }
        for (((value, &gradient), moment), velocity) in parameter
            .iter_mut()
            .zip(gradient)
            .zip(&mut self.moment)
            .zip(&mut self.velocity)
        {
            let gradient = gradient * clip;
            *moment = self.setting.first * *moment + (1.0 - self.setting.first) * gradient;
            *velocity =
                self.setting.second * *velocity + (1.0 - self.setting.second) * gradient * gradient;
            *value -=
                rate * (*moment / first) / ((*velocity / second).sqrt() + self.setting.epsilon);
        }
        norm
    }
}
