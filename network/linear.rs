use crate::layout::{Kind, Layout};
use crate::matrix::{Span, accumulate, product};
use ndarray::{Array2, ArrayView2, Axis};
use random::Generator;

#[derive(Clone, Debug)]
pub struct Linear {
    pub weight: Span,
    pub bias: Span,
}

impl Linear {
    pub(crate) fn new(layout: &mut Layout, input: usize, output: usize) -> Self {
        Self {
            weight: layout.allocate(input, output, Kind::Weight),
            bias: layout.allocate(1, output, Kind::Bias),
        }
    }

    pub(crate) fn initialize(
        &self,
        parameter: &mut [f32],
        generator: &mut Generator,
        deviation: f32,
    ) {
        for value in self.weight.write(parameter) {
            *value = generator.normal() as f32 * deviation;
        }
        self.bias.write(parameter).fill(0.0);
    }

    pub(crate) fn forward(&self, parameter: &[f32], input: &ArrayView2<'_, f32>) -> Array2<f32> {
        let mut result = product(input, &self.weight.view(parameter));
        result += &self.bias.view(parameter).row(0);
        result
    }

    pub(crate) fn backward(
        &self,
        parameter: &[f32],
        gradient: &mut [f32],
        input: &ArrayView2<'_, f32>,
        delta: &ArrayView2<'_, f32>,
    ) -> Array2<f32> {
        accumulate(&mut self.weight.edit(gradient), &input.t(), delta);
        let mut bias = self.bias.edit(gradient);
        bias.row_mut(0).scaled_add(1.0, &delta.sum_axis(Axis(0)));
        product(delta, &self.weight.view(parameter).t())
    }
}
