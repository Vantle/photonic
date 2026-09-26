use crate::block::{self, Block};
use crate::configuration::Configuration;
use crate::embedding::Embedding;
use crate::head::Head;
use crate::input::{Input, Output, Sample};
use crate::layout::Layout;
use crate::loss::{Loss, Weight, measure};
use crate::norm::Norm;
use crate::pack::Pack;
use ndarray::Array2;
use random::Generator;
use std::ops::Range;
use thiserror::Error;

const DEVIATION: f32 = 0.02;

#[derive(Debug, Error, PartialEq)]
#[error("the model holds {expected} parameters but {found} were given")]
pub struct Mismatch {
    pub expected: usize,
    pub found: usize,
}

#[derive(Clone, Debug)]
pub struct Model {
    configuration: Configuration,
    layout: Layout,
    embedding: Embedding,
    block: Vec<Block>,
    norm: Norm,
    head: Head,
    parameter: Vec<f32>,
}

fn sum(part: &[usize]) -> Option<usize> {
    part.iter()
        .try_fold(0usize, |total, &value| total.checked_add(value))
}

fn linear(input: usize, output: usize) -> Option<usize> {
    input.checked_mul(output)?.checked_add(output)
}

impl Model {
    fn structure(configuration: &Configuration) -> (Layout, Embedding, Vec<Block>, Norm, Head) {
        let mut layout = Layout::default();
        let embedding = Embedding::new(&mut layout, &configuration.field, configuration.width);
        let block = (0..configuration.depth)
            .map(|_| {
                Block::new(
                    &mut layout,
                    configuration.width,
                    configuration.head,
                    configuration.hidden,
                )
            })
            .collect();
        let norm = Norm::new(&mut layout, configuration.width);
        let head = Head::new(&mut layout, configuration);
        (layout, embedding, block, norm, head)
    }

    pub fn new(configuration: Configuration, generator: &mut Generator) -> Self {
        let (layout, embedding, block, norm, head) = Self::structure(&configuration);
        let mut parameter = vec![0.0; layout.total()];
        let residual = DEVIATION / (2.0 * configuration.depth as f32).sqrt();
        embedding.initialize(&mut parameter, generator, DEVIATION);
        for block in &block {
            block.initialize(&mut parameter, generator, DEVIATION, residual);
        }
        norm.initialize(&mut parameter);
        head.initialize(&mut parameter, generator, DEVIATION);
        Self {
            configuration,
            layout,
            embedding,
            block,
            norm,
            head,
            parameter,
        }
    }

    pub(crate) fn length(configuration: &Configuration) -> Option<usize> {
        let width = configuration.width;
        let norm = width.checked_mul(2)?;
        let pointer = width.checked_mul(configuration.binary.checked_mul(configuration.key)?)?;
        let embedding = configuration
            .field
            .iter()
            .try_fold(0usize, |total, &cardinality| {
                total.checked_add(cardinality.checked_mul(width)?)
            })?;
        let block = sum(&[
            norm,
            linear(width, width.checked_mul(3)?)?,
            linear(width, width)?,
            norm,
            linear(width, configuration.hidden)?,
            linear(configuration.hidden, width)?,
        ])?;
        let head = sum(&[
            linear(width, width)?,
            linear(width, 1)?,
            linear(width, configuration.unary)?,
            pointer,
            pointer,
            linear(width, configuration.judge)?,
        ])?;
        sum(&[
            embedding,
            block.checked_mul(configuration.depth)?,
            norm,
            head,
        ])
    }

    pub(crate) fn restore(configuration: Configuration, parameter: Vec<f32>) -> Option<Self> {
        let (layout, embedding, block, norm, head) = Self::structure(&configuration);
        (layout.total() == parameter.len()).then_some(Self {
            configuration,
            layout,
            embedding,
            block,
            norm,
            head,
            parameter,
        })
    }

    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }

    pub fn embedding(&self) -> &Embedding {
        &self.embedding
    }

    pub fn block(&self) -> &[Block] {
        &self.block
    }

    pub fn norm(&self) -> &Norm {
        &self.norm
    }

    pub fn head(&self) -> &Head {
        &self.head
    }

    pub fn size(&self) -> usize {
        self.parameter.len()
    }

    pub fn parameter(&self) -> &[f32] {
        &self.parameter
    }

    pub fn edit(&mut self) -> &mut [f32] {
        &mut self.parameter
    }

    pub fn load(&mut self, parameter: Vec<f32>) -> Result<(), Mismatch> {
        if parameter.len() != self.parameter.len() {
            return Err(Mismatch {
                expected: self.parameter.len(),
                found: parameter.len(),
            });
        }
        self.parameter = parameter;
        Ok(())
    }

    pub fn decay(&self) -> &[Range<usize>] {
        self.layout.decay()
    }

    pub fn infer(&self, input: &[&Input]) -> Vec<Output> {
        if input.is_empty() {
            return Vec::new();
        }
        let pack = Pack::new(input, self.configuration.field.len());
        let mut state =
            self.embedding
                .forward(&self.parameter, &pack.feature, self.configuration.width);
        for block in &self.block {
            state = block.forward(&self.parameter, state, &pack.boundary).0;
        }
        let (encoded, _) = self.norm.forward(&self.parameter, &state.view());
        self.head.forward(&self.parameter, &encoded.view(), &pack).0
    }

    pub fn gradient(&self, sample: &[&Sample], gradient: &mut [f32], weight: Weight) -> Loss {
        let input = sample
            .iter()
            .map(|sample| &sample.input)
            .collect::<Vec<_>>();
        let pack = Pack::new(&input, self.configuration.field.len());
        let mut state =
            self.embedding
                .forward(&self.parameter, &pack.feature, self.configuration.width);
        let mut trace: Vec<block::Trace> = Vec::with_capacity(self.block.len());
        for block in &self.block {
            let (next, record) = block.forward(&self.parameter, state, &pack.boundary);
            state = next;
            trace.push(record);
        }
        let (encoded, norm) = self.norm.forward(&self.parameter, &state.view());
        let (output, head) = self.head.forward(&self.parameter, &encoded.view(), &pack);
        let (loss, change) = measure(&output, sample, weight);
        let delta = self.head.backward(
            &self.parameter,
            gradient,
            &encoded.view(),
            &pack,
            &head,
            &change,
        );
        let mut delta: Array2<f32> =
            self.norm
                .backward(&self.parameter, gradient, &norm, &delta.view());
        for (block, record) in self.block.iter().zip(&trace).rev() {
            delta = block.backward(&self.parameter, gradient, record, delta, &pack.boundary);
        }
        self.embedding
            .backward(gradient, &pack.feature, &delta.view());
        loss
    }
}
