use crate::configuration::Configuration;
use crate::linear::Linear;
use crate::matrix::Span;
use crate::model::Model;
use crate::norm::Norm;
use random::Generator;
use thiserror::Error;

const NOISE: f32 = 1e-2;

#[derive(Debug, Error, PartialEq)]
pub enum Failure {
    #[error("the width grows only by whole multiples, not from {from} to {to}")]
    Width { from: usize, to: usize },
    #[error("attention heads keep their dimension of {dimension}")]
    Head { dimension: usize },
    #[error("a network never shrinks")]
    Shrink,
    #[error("input fields may only be added or widened, and the pointer width must stay the same")]
    Interface,
}

#[derive(Clone, Copy)]
enum Axis {
    Residual,
    Part,
    Unit,
    Same,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Fresh {
    Keep,
    Zero,
}

struct Growth<'source> {
    source: &'source [f32],
    width: usize,
    factor: usize,
}

fn check(from: &Configuration, to: &Configuration) -> Result<usize, Failure> {
    if from.key != to.key || to.field.len() < from.field.len() {
        return Err(Failure::Interface);
    }
    if from
        .field
        .iter()
        .zip(&to.field)
        .any(|(before, after)| after < before)
        || to.width < from.width
        || to.depth < from.depth
        || to.hidden < from.hidden
        || to.unary < from.unary
        || to.binary < from.binary
        || to.judge < from.judge
    {
        return Err(Failure::Shrink);
    }
    if !to.width.is_multiple_of(from.width) {
        return Err(Failure::Width {
            from: from.width,
            to: to.width,
        });
    }
    let factor = to.width / from.width;
    if to.head != from.head * factor {
        return Err(Failure::Head {
            dimension: from.dimension(),
        });
    }
    Ok(factor)
}

impl Growth<'_> {
    fn map(&self, axis: Axis, index: usize, size: usize) -> Option<usize> {
        match axis {
            Axis::Residual => Some(index % self.width),
            Axis::Part => {
                let part = self.width * self.factor;
                Some(index / part * self.width + index % part % self.width)
            }
            Axis::Unit => (index < size).then_some(index),
            Axis::Same => Some(index),
        }
    }

    fn copy(
        &self,
        target: &mut [f32],
        span: [Span; 2],
        axis: [Axis; 2],
        fresh: Fresh,
        generator: &mut Generator,
    ) {
        let [before, after] = span;
        let split = if matches!(axis[0], Axis::Residual) {
            self.factor
        } else {
            1
        };
        let source = before.read(self.source);
        let scale = (source.iter().map(|value| value * value).sum::<f32>()
            / source.len().max(1) as f32)
            .sqrt()
            * NOISE;
        for row in 0..after.row {
            let origin = self.map(axis[0], row, before.row);
            for column in 0..after.column {
                let place = after.start + row * after.column + column;
                match (origin, self.map(axis[1], column, before.column)) {
                    (Some(origin), Some(other)) => {
                        target[place] = source[origin * before.column + other] / split as f32;
                    }
                    _ if fresh == Fresh::Zero => target[place] = 0.0,
                    _ => {}
                }
            }
        }
        if split == 1 {
            return;
        }
        for row in 0..self.width {
            for column in 0..after.column {
                let noise = (0..split)
                    .map(|_| generator.normal() as f32 * scale)
                    .collect::<Vec<_>>();
                let mean = noise.iter().sum::<f32>() / split as f32;
                for (copy, value) in noise.iter().enumerate() {
                    target[after.start + (row + copy * self.width) * after.column + column] +=
                        value - mean;
                }
            }
        }
    }

    fn linear(
        &self,
        target: &mut [f32],
        pair: [&Linear; 2],
        axis: [Axis; 2],
        fresh: Fresh,
        generator: &mut Generator,
    ) {
        let [before, after] = pair;
        self.copy(
            target,
            [before.weight, after.weight],
            axis,
            fresh,
            generator,
        );
        self.copy(
            target,
            [before.bias, after.bias],
            [Axis::Same, axis[1]],
            fresh,
            generator,
        );
    }

    fn norm(&self, target: &mut [f32], pair: [&Norm; 2], generator: &mut Generator) {
        let [before, after] = pair;
        for span in [[before.scale, after.scale], [before.shift, after.shift]] {
            self.copy(
                target,
                span,
                [Axis::Same, Axis::Residual],
                Fresh::Keep,
                generator,
            );
        }
    }
}

fn silence(target: &mut [f32], linear: &Linear) {
    linear.weight.write(target).fill(0.0);
    linear.bias.write(target).fill(0.0);
}

pub fn grow(
    source: &Model,
    configuration: Configuration,
    generator: &mut Generator,
) -> Result<Model, Failure> {
    let factor = check(source.configuration(), &configuration)?;
    let growth = Growth {
        source: source.parameter(),
        width: source.configuration().width,
        factor,
    };
    let mut model = Model::new(configuration, generator);
    let mut target = model.parameter().to_vec();
    for (index, after) in model.embedding().table.iter().enumerate() {
        let Some(before) = source.embedding().table.get(index) else {
            after.write(&mut target).fill(0.0);
            continue;
        };
        growth.copy(
            &mut target,
            [*before, *after],
            [Axis::Unit, Axis::Residual],
            Fresh::Zero,
            generator,
        );
    }
    for (index, block) in model.block().iter().enumerate() {
        let Some(before) = source.block().get(index) else {
            silence(&mut target, &block.attention.output);
            silence(&mut target, &block.feed.contract);
            continue;
        };
        growth.norm(&mut target, [&before.first, &block.first], generator);
        growth.linear(
            &mut target,
            [&before.attention.project, &block.attention.project],
            [Axis::Residual, Axis::Part],
            Fresh::Keep,
            generator,
        );
        growth.linear(
            &mut target,
            [&before.attention.output, &block.attention.output],
            [Axis::Residual, Axis::Residual],
            Fresh::Keep,
            generator,
        );
        growth.norm(&mut target, [&before.second, &block.second], generator);
        growth.linear(
            &mut target,
            [&before.feed.expand, &block.feed.expand],
            [Axis::Residual, Axis::Unit],
            Fresh::Keep,
            generator,
        );
        growth.linear(
            &mut target,
            [&before.feed.contract, &block.feed.contract],
            [Axis::Unit, Axis::Residual],
            Fresh::Zero,
            generator,
        );
    }
    growth.norm(&mut target, [source.norm(), model.norm()], generator);
    let (before, after) = (source.head(), model.head());
    growth.linear(
        &mut target,
        [&before.value, &after.value],
        [Axis::Residual, Axis::Residual],
        Fresh::Keep,
        generator,
    );
    growth.linear(
        &mut target,
        [&before.score, &after.score],
        [Axis::Residual, Axis::Same],
        Fresh::Keep,
        generator,
    );
    for pair in [[&before.unary, &after.unary], [&before.judge, &after.judge]] {
        growth.linear(
            &mut target,
            pair,
            [Axis::Residual, Axis::Unit],
            Fresh::Zero,
            generator,
        );
    }
    for (span, fresh) in [
        ([before.query, after.query], Fresh::Zero),
        ([before.key, after.key], Fresh::Keep),
    ] {
        growth.copy(
            &mut target,
            span,
            [Axis::Residual, Axis::Unit],
            fresh,
            generator,
        );
    }
    model.edit().copy_from_slice(&target);
    Ok(model)
}
