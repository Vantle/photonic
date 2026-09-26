use crate::layout::Layout;
use crate::linear::Linear;
use ndarray::{Array2, ArrayView2, Zip};
use random::Generator;

const ROOT: f32 = 0.797_884_6;
const CUBIC: f32 = 0.044_715;

fn tangent(value: f32) -> (f32, f32) {
    if value.abs() >= 3.0 {
        return (value.signum(), 0.0);
    }
    let square = value * value;
    let denominator = 27.0 + 9.0 * square;
    let result = value * (27.0 + square) / denominator;
    let derivative = ((27.0 + 3.0 * square) * denominator - value * (27.0 + square) * 18.0 * value)
        / (denominator * denominator);
    (result, derivative)
}

pub(crate) fn gelu(value: f32) -> f32 {
    0.5 * value * (1.0 + tangent(ROOT * (value + CUBIC * value * value * value)).0)
}

pub(crate) fn slope(value: f32) -> f32 {
    let (result, derivative) = tangent(ROOT * (value + CUBIC * value * value * value));
    0.5 * (1.0 + result) + 0.5 * value * derivative * ROOT * (1.0 + 3.0 * CUBIC * value * value)
}

#[derive(Clone, Debug)]
pub struct Feed {
    pub expand: Linear,
    pub contract: Linear,
}

#[derive(Clone, Debug)]
pub(crate) struct Trace {
    input: Array2<f32>,
    activation: Array2<f32>,
    hidden: Array2<f32>,
}

impl Feed {
    pub(crate) fn new(layout: &mut Layout, width: usize, hidden: usize) -> Self {
        Self {
            expand: Linear::new(layout, width, hidden),
            contract: Linear::new(layout, hidden, width),
        }
    }

    pub(crate) fn initialize(
        &self,
        parameter: &mut [f32],
        generator: &mut Generator,
        deviation: f32,
        residual: f32,
    ) {
        self.expand.initialize(parameter, generator, deviation);
        self.contract.initialize(parameter, generator, residual);
    }

    pub(crate) fn forward(&self, parameter: &[f32], input: Array2<f32>) -> (Array2<f32>, Trace) {
        let activation = self.expand.forward(parameter, &input.view());
        let hidden = activation.mapv(gelu);
        let output = self.contract.forward(parameter, &hidden.view());
        (
            output,
            Trace {
                input,
                activation,
                hidden,
            },
        )
    }

    pub(crate) fn backward(
        &self,
        parameter: &[f32],
        gradient: &mut [f32],
        trace: &Trace,
        delta: &ArrayView2<'_, f32>,
    ) -> Array2<f32> {
        let mut hidden = self
            .contract
            .backward(parameter, gradient, &trace.hidden.view(), delta);
        Zip::from(&mut hidden)
            .and(&trace.activation)
            .for_each(|delta, &activation| *delta *= slope(activation));
        self.expand
            .backward(parameter, gradient, &trace.input.view(), &hidden.view())
    }
}
