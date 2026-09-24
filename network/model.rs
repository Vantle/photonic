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

const DEVIATION: f32 = 0.02;

#[derive(Clone, Debug)]
pub struct Model {
    configuration: Configuration,
    layout: Layout,
    embedding: Embedding,
    block: Vec<Block>,
    norm: Norm,
    head: Head,
    pub parameter: Vec<f32>,
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

    pub fn length(configuration: &Configuration) -> usize {
        Self::structure(configuration).0.total()
    }

    pub fn restore(configuration: Configuration, parameter: Vec<f32>) -> Option<Self> {
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
