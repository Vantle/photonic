use crate::layout::{Kind, Layout};
use crate::matrix::Span;
use ndarray::{Array2, ArrayView2, Zip};

const EPSILON: f32 = 1e-5;

#[derive(Clone, Debug)]
pub struct Norm {
    pub scale: Span,
    pub shift: Span,
}

#[derive(Clone, Debug)]
pub struct Trace {
    normal: Array2<f32>,
    deviation: Vec<f32>,
}

impl Norm {
    pub fn new(layout: &mut Layout, width: usize) -> Self {
        Self {
            scale: layout.allocate(1, width, Kind::Scale),
            shift: layout.allocate(1, width, Kind::Bias),
        }
    }

    pub fn initialize(&self, parameter: &mut [f32]) {
        self.scale.write(parameter).fill(1.0);
        self.shift.write(parameter).fill(0.0);
    }

    pub fn forward(&self, parameter: &[f32], input: &ArrayView2<'_, f32>) -> (Array2<f32>, Trace) {
        let width = input.ncols() as f32;
        let mut normal = input.to_owned();
        let mut deviation = Vec::with_capacity(input.nrows());
        for mut row in normal.rows_mut() {
            let mean = row.sum() / width;
            let variance = row.iter().map(|value| (value - mean).powi(2)).sum::<f32>() / width;
            let inverse = 1.0 / (variance + EPSILON).sqrt();
            row.mapv_inplace(|value| (value - mean) * inverse);
            deviation.push(inverse);
        }
        let scale = self.scale.view(parameter);
        let shift = self.shift.view(parameter);
        let mut output = normal.clone();
        output *= &scale.row(0);
        output += &shift.row(0);
        (output, Trace { normal, deviation })
    }

    pub fn backward(
        &self,
        parameter: &[f32],
        gradient: &mut [f32],
        trace: &Trace,
        delta: &ArrayView2<'_, f32>,
    ) -> Array2<f32> {
        let width = delta.ncols() as f32;
        {
            let mut scale = self.scale.edit(gradient);
            let mut row = scale.row_mut(0);
            for (delta, normal) in delta.rows().into_iter().zip(trace.normal.rows()) {
                Zip::from(&mut row)
                    .and(&delta)
                    .and(&normal)
                    .for_each(|target, &delta, &normal| *target += delta * normal);
            }
        }
        {
            let mut shift = self.shift.edit(gradient);
            let mut row = shift.row_mut(0);
            for delta in delta.rows() {
                row += &delta;
            }
        }
        let scale = self.scale.view(parameter);
        let mut result = Array2::zeros(delta.raw_dim());
        for (((mut target, delta), normal), &inverse) in result
            .rows_mut()
            .into_iter()
            .zip(delta.rows())
            .zip(trace.normal.rows())
            .zip(&trace.deviation)
        {
            let scaled = &delta * &scale.row(0);
            let mean = scaled.sum() / width;
            let projection = scaled
                .iter()
                .zip(normal.iter())
                .map(|(a, b)| a * b)
                .sum::<f32>()
                / width;
            Zip::from(&mut target).and(&scaled).and(&normal).for_each(
                |target, &scaled, &normal| {
                    *target = inverse * (scaled - mean - normal * projection);
                },
            );
        }
        result
    }
}
