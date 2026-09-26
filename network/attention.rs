use crate::layout::Layout;
use crate::linear::Linear;
use crate::matrix::{edit, product, view};
use ndarray::linalg::general_mat_mul;
use ndarray::{Array2, ArrayView2};
use random::Generator;
use std::ops::Range;

#[derive(Clone, Debug)]
pub struct Attention {
    pub project: Linear,
    pub output: Linear,
    head: usize,
    width: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Trace {
    input: Array2<f32>,
    mixed: Array2<f32>,
    weight: Vec<Array2<f32>>,
    context: Array2<f32>,
}

fn softmax(score: &mut Array2<f32>) {
    for mut row in score.rows_mut() {
        let maximum = row.fold(f32::NEG_INFINITY, |left, &right| left.max(right));
        row.mapv_inplace(|value| (value - maximum).exp());
        let total = row.sum();
        row.mapv_inplace(|value| value / total);
    }
}

impl Attention {
    pub(crate) fn new(layout: &mut Layout, width: usize, head: usize) -> Self {
        Self {
            project: Linear::new(layout, width, 3 * width),
            output: Linear::new(layout, width, width),
            head,
            width,
        }
    }

    pub(crate) fn initialize(
        &self,
        parameter: &mut [f32],
        generator: &mut Generator,
        deviation: f32,
        residual: f32,
    ) {
        self.project.initialize(parameter, generator, deviation);
        self.output.initialize(parameter, generator, residual);
    }

    fn dimension(&self) -> usize {
        self.width / self.head
    }

    fn column(&self, part: usize, head: usize) -> Range<usize> {
        let start = part * self.width + head * self.dimension();
        start..start + self.dimension()
    }

    pub(crate) fn forward(
        &self,
        parameter: &[f32],
        input: Array2<f32>,
        boundary: &[Range<usize>],
    ) -> (Array2<f32>, Trace) {
        let mixed = self.project.forward(parameter, &input.view());
        let scale = 1.0 / (self.dimension() as f32).sqrt();
        let mut context = Array2::zeros((input.nrows(), self.width));
        let mut weight = Vec::with_capacity(boundary.len() * self.head);
        for range in boundary {
            for head in 0..self.head {
                let query = view(&mixed, range.clone(), self.column(0, head));
                let key = view(&mixed, range.clone(), self.column(1, head));
                let value = view(&mixed, range.clone(), self.column(2, head));
                let mut score = product(&query, &key.t());
                score *= scale;
                softmax(&mut score);
                let mut target = edit(&mut context, range.clone(), self.column(0, head));
                general_mat_mul(1.0, &score, &value, 0.0, &mut target);
                weight.push(score);
            }
        }
        let output = self.output.forward(parameter, &context.view());
        (
            output,
            Trace {
                input,
                mixed,
                weight,
                context,
            },
        )
    }

    pub(crate) fn backward(
        &self,
        parameter: &[f32],
        gradient: &mut [f32],
        trace: &Trace,
        delta: &ArrayView2<'_, f32>,
        boundary: &[Range<usize>],
    ) -> Array2<f32> {
        let context = self
            .output
            .backward(parameter, gradient, &trace.context.view(), delta);
        let scale = 1.0 / (self.dimension() as f32).sqrt();
        let mut mixed = Array2::zeros(trace.mixed.raw_dim());
        let mut weight = trace.weight.iter();
        for range in boundary {
            for head in 0..self.head {
                let probability = weight.next().expect("one weight per sample head");
                let query = view(&trace.mixed, range.clone(), self.column(0, head));
                let key = view(&trace.mixed, range.clone(), self.column(1, head));
                let value = view(&trace.mixed, range.clone(), self.column(2, head));
                let outer = view(&context, range.clone(), self.column(0, head));
                let mut score = product(&outer, &value.t());
                for (mut row, probability) in score.rows_mut().into_iter().zip(probability.rows()) {
                    let inner = row
                        .iter()
                        .zip(probability.iter())
                        .map(|(one, other)| one * other)
                        .sum::<f32>();
                    row.zip_mut_with(&probability, |delta, &probability| {
                        *delta = probability * (*delta - inner) * scale;
                    });
                }
                general_mat_mul(
                    1.0,
                    &probability.t(),
                    &outer,
                    1.0,
                    &mut edit(&mut mixed, range.clone(), self.column(2, head)),
                );
                general_mat_mul(
                    1.0,
                    &score,
                    &key,
                    1.0,
                    &mut edit(&mut mixed, range.clone(), self.column(0, head)),
                );
                general_mat_mul(
                    1.0,
                    &score.t(),
                    &query,
                    1.0,
                    &mut edit(&mut mixed, range.clone(), self.column(1, head)),
                );
            }
        }
        self.project
            .backward(parameter, gradient, &trace.input.view(), &mixed.view())
    }
}
