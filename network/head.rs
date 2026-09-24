use crate::configuration::Configuration;
use crate::feed::{gelu, slope};
use crate::input::{Output, Pointer};
use crate::layout::{Kind, Layout};
use crate::linear::Linear;
use crate::matrix::{Span, accumulate, edit, product, view};
use crate::pack::Pack;
use ndarray::{Array2, ArrayView2, Zip};
use random::Generator;

#[derive(Clone, Debug)]
pub struct Head {
    pub value: Linear,
    pub score: Linear,
    pub unary: Linear,
    pub query: Span,
    pub key: Span,
    pub judge: Linear,
    binary: usize,
    dimension: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Delta {
    pub logit: Vec<Vec<f32>>,
    pub value: Vec<f32>,
    pub judge: Vec<Vec<f32>>,
}

#[derive(Clone, Debug)]
pub struct Trace {
    summary: Array2<f32>,
    activation: Array2<f32>,
    hidden: Array2<f32>,
    unary: Array2<f32>,
    query: Array2<f32>,
    key: Array2<f32>,
}

impl Head {
    pub fn new(layout: &mut Layout, configuration: &Configuration) -> Self {
        let width = configuration.width;
        let (binary, dimension) = (configuration.binary, configuration.key);
        Self {
            value: Linear::new(layout, width, width),
            score: Linear::new(layout, width, 1),
            unary: Linear::new(layout, width, configuration.unary),
            query: layout.allocate(width, binary * dimension, Kind::Weight),
            key: layout.allocate(width, binary * dimension, Kind::Weight),
            judge: Linear::new(layout, width, configuration.judge),
            binary,
            dimension,
        }
    }

    pub fn initialize(&self, parameter: &mut [f32], generator: &mut Generator, deviation: f32) {
        self.value.initialize(parameter, generator, deviation);
        self.score.initialize(parameter, generator, 0.0);
        self.unary.initialize(parameter, generator, 0.0);
        for span in [self.query, self.key] {
            for value in span.write(parameter) {
                *value = generator.normal() as f32 * deviation;
            }
        }
        self.judge.initialize(parameter, generator, 0.0);
    }

    fn columns(&self, head: u16) -> std::ops::Range<usize> {
        let start = usize::from(head) * self.dimension;
        start..start + self.dimension
    }

    fn scale(&self) -> f32 {
        1.0 / (self.dimension as f32).sqrt()
    }

    pub fn forward(
        &self,
        parameter: &[f32],
        encoded: &ArrayView2<'_, f32>,
        pack: &Pack,
    ) -> (Vec<Output>, Trace) {
        let mut summary = Array2::zeros((pack.boundary.len(), encoded.ncols()));
        for (mut row, range) in summary.rows_mut().into_iter().zip(&pack.boundary) {
            row.assign(&encoded.row(range.start));
        }
        let activation = self.value.forward(parameter, &summary.view());
        let hidden = activation.mapv(gelu);
        let value = self.score.forward(parameter, &hidden.view());
        let judge = self.judge.forward(parameter, &hidden.view());
        let unary = self.unary.forward(parameter, encoded);
        let (query, key) = if self.binary > 0 {
            (
                product(encoded, &self.query.view(parameter)),
                product(encoded, &self.key.view(parameter)),
            )
        } else {
            (Array2::zeros((0, 0)), Array2::zeros((0, 0)))
        };
        let scale = self.scale();
        let output = pack
            .pointer
            .iter()
            .enumerate()
            .map(|(sample, pointer)| Output {
                logit: pointer
                    .iter()
                    .map(|pointer| match *pointer {
                        Pointer::Unary { head, token } => {
                            unary[[token as usize, usize::from(head)]]
                        }
                        Pointer::Binary { head, left, right } => {
                            let columns = self.columns(head);
                            let (left, right) = (left as usize, right as usize);
                            view(&query, left..left + 1, columns.clone())
                                .iter()
                                .zip(view(&key, right..right + 1, columns).iter())
                                .map(|(a, b)| a * b)
                                .sum::<f32>()
                                * scale
                        }
                    })
                    .collect(),
                value: value[[sample, 0]],
                judge: judge.row(sample).to_vec(),
            })
            .collect();
        (
            output,
            Trace {
                summary,
                activation,
                hidden,
                unary,
                query,
                key,
            },
        )
    }

    pub fn backward(
        &self,
        parameter: &[f32],
        gradient: &mut [f32],
        encoded: &ArrayView2<'_, f32>,
        pack: &Pack,
        trace: &Trace,
        delta: &Delta,
    ) -> Array2<f32> {
        let mut result = Array2::zeros(encoded.raw_dim());
        let score = Array2::from_shape_vec((delta.value.len(), 1), delta.value.clone())
            .expect("one value delta per sample");
        let mut hidden =
            self.score
                .backward(parameter, gradient, &trace.hidden.view(), &score.view());
        let width = self.judge.weight.column;
        let judge = Array2::from_shape_vec(
            (delta.judge.len(), width),
            delta.judge.iter().flatten().copied().collect(),
        )
        .expect("one judge delta per sample");
        hidden += &self
            .judge
            .backward(parameter, gradient, &trace.hidden.view(), &judge.view());
        Zip::from(&mut hidden)
            .and(&trace.activation)
            .for_each(|delta, &activation| *delta *= slope(activation));
        let summary =
            self.value
                .backward(parameter, gradient, &trace.summary.view(), &hidden.view());
        for (row, range) in summary.rows().into_iter().zip(&pack.boundary) {
            result.row_mut(range.start).scaled_add(1.0, &row);
        }
        let mut unary = Array2::zeros(trace.unary.raw_dim());
        let mut query = Array2::zeros(trace.query.raw_dim());
        let mut key = Array2::zeros(trace.key.raw_dim());
        let scale = self.scale();
        for (pointer, delta) in pack.pointer.iter().zip(&delta.logit) {
            for (pointer, &delta) in pointer.iter().zip(delta) {
                match *pointer {
                    Pointer::Unary { head, token } => {
                        unary[[token as usize, usize::from(head)]] += delta;
                    }
                    Pointer::Binary { head, left, right } => {
                        let columns = self.columns(head);
                        let (left, right) = (left as usize, right as usize);
                        let source = view(&trace.key, right..right + 1, columns.clone());
                        edit(&mut query, left..left + 1, columns.clone())
                            .scaled_add(delta * scale, &source);
                        let source = view(&trace.query, left..left + 1, columns.clone());
                        edit(&mut key, right..right + 1, columns)
                            .scaled_add(delta * scale, &source);
                    }
                }
            }
        }
        result += &self
            .unary
            .backward(parameter, gradient, encoded, &unary.view());
        if self.binary > 0 {
            accumulate(&mut self.query.edit(gradient), &encoded.t(), &query.view());
            accumulate(&mut self.key.edit(gradient), &encoded.t(), &key.view());
            accumulate(
                &mut result.view_mut(),
                &query.view(),
                &self.query.view(parameter).t(),
            );
            accumulate(
                &mut result.view_mut(),
                &key.view(),
                &self.key.view(parameter).t(),
            );
        }
        result
    }
}
