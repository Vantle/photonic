use crate::attention::{self, Attention};
use crate::feed::{self, Feed};
use crate::layout::Layout;
use crate::norm::{self, Norm};
use ndarray::Array2;
use random::Generator;
use std::ops::Range;

#[derive(Clone, Debug)]
pub struct Block {
    pub first: Norm,
    pub attention: Attention,
    pub second: Norm,
    pub feed: Feed,
}

#[derive(Clone, Debug)]
pub struct Trace {
    first: norm::Trace,
    attention: attention::Trace,
    second: norm::Trace,
    feed: feed::Trace,
}

impl Block {
    pub fn new(layout: &mut Layout, width: usize, head: usize, hidden: usize) -> Self {
        Self {
            first: Norm::new(layout, width),
            attention: Attention::new(layout, width, head),
            second: Norm::new(layout, width),
            feed: Feed::new(layout, width, hidden),
        }
    }

    pub fn initialize(
        &self,
        parameter: &mut [f32],
        generator: &mut Generator,
        deviation: f32,
        residual: f32,
    ) {
        self.first.initialize(parameter);
        self.attention
            .initialize(parameter, generator, deviation, residual);
        self.second.initialize(parameter);
        self.feed
            .initialize(parameter, generator, deviation, residual);
    }

    pub fn forward(
        &self,
        parameter: &[f32],
        input: Array2<f32>,
        boundary: &[Range<usize>],
    ) -> (Array2<f32>, Trace) {
        let (normal, first) = self.first.forward(parameter, &input.view());
        let (mixed, attention) = self.attention.forward(parameter, normal, boundary);
        let middle = input + mixed;
        let (normal, second) = self.second.forward(parameter, &middle.view());
        let (fed, feed) = self.feed.forward(parameter, normal);
        (
            middle + fed,
            Trace {
                first,
                attention,
                second,
                feed,
            },
        )
    }

    pub fn backward(
        &self,
        parameter: &[f32],
        gradient: &mut [f32],
        trace: &Trace,
        delta: Array2<f32>,
        boundary: &[Range<usize>],
    ) -> Array2<f32> {
        let fed = self
            .feed
            .backward(parameter, gradient, &trace.feed, &delta.view());
        let middle = delta
            + self
                .second
                .backward(parameter, gradient, &trace.second, &fed.view());
        let mixed = self.attention.backward(
            parameter,
            gradient,
            &trace.attention,
            &middle.view(),
            boundary,
        );
        middle
            + self
                .first
                .backward(parameter, gradient, &trace.first, &mixed.view())
    }
}
