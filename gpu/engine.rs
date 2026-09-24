use crate::adam::Adam;
use crate::batch::{Batch, Extent};
use crate::delta::Delta;
use crate::failure::Failure;
use crate::occurrence::Occurrence;
use crate::pipeline::{Pipeline, dispatch, tiled};
use crate::runtime::{Command, Device, Memory, Operand, Product};
use crate::trace::{Stage, Trace};
use network::input::{Input, Output, Sample};
use network::linear::Linear;
use network::loss::{Loss, Weight, measure};
use network::matrix::Span;
use network::model::Model;
use network::norm::Norm;
use network::optimizer::Optimizer;

const BLOCK: usize = 256;
const CAPACITY: usize = 128;

pub struct Engine {
    device: Device,
    pipeline: Pipeline,
    model: Model,
    parameter: Memory,
    table: Memory,
    target: Memory,
    inference: Option<Trace>,
    training: Option<(Trace, Delta)>,
    adam: Option<Adam>,
}

struct Layer<'memory> {
    input: &'memory Memory,
    row: usize,
    linear: &'memory Linear,
    output: &'memory Memory,
    accumulate: bool,
}

fn operand(
    memory: &Memory,
    offset: usize,
    row: usize,
    column: usize,
    transpose: bool,
) -> Operand<'_> {
    Operand {
        memory,
        offset,
        row,
        column,
        transpose,
    }
}

fn read(batch: &Batch, trace: &mut Trace, judge: usize) -> Vec<Output> {
    let logit = trace.logit.view::<f32>();
    let score = trace.score.view::<f32>();
    let verdict = trace.judge.view::<f32>();
    let mut cursor = 0;
    batch
        .pack
        .pointer
        .iter()
        .enumerate()
        .map(|(index, pointer)| {
            let output = Output {
                logit: logit[cursor..cursor + pointer.len()].to_vec(),
                value: score[index],
                judge: verdict[index * judge..(index + 1) * judge].to_vec(),
            };
            cursor += pointer.len();
            output
        })
        .collect()
}

impl Engine {
    pub fn new(model: &Model) -> Result<Self, Failure> {
        if model.configuration().dimension() > CAPACITY {
            return Err(Failure::Kernel(format!(
                "attention heads wider than {CAPACITY} exceed the kernel"
            )));
        }
        let device = Device::open()?;
        let table = &model.embedding().table;
        let start = table
            .iter()
            .map(|span| span.start as u32)
            .collect::<Vec<_>>();
        let target = table
            .iter()
            .flat_map(|span| (0..span.row).map(|value| (span.start + value * span.column) as u32))
            .collect::<Vec<_>>();
        Ok(Self {
            pipeline: Pipeline::new(&device, model.configuration().dimension())?,
            parameter: device.upload(&model.parameter)?,
            table: device.upload(&start)?,
            target: device.upload(&target)?,
            device,
            model: model.clone(),
            inference: None,
            training: None,
            adam: None,
        })
    }

    pub fn name(&self) -> &str {
        self.device.name()
    }

    pub fn load(&mut self, parameter: &[f32]) {
        self.parameter.edit::<f32>().copy_from_slice(parameter);
    }

    pub fn parameter(&mut self) -> Vec<f32> {
        self.parameter.view::<f32>().to_vec()
    }

    pub fn prepare(&mut self, optimizer: &Optimizer) -> Result<(), Failure> {
        self.adam = Some(Adam::new(&self.device, &self.model, optimizer)?);
        Ok(())
    }

    pub fn optimizer(&mut self) -> Option<Optimizer> {
        self.adam.as_mut().map(Adam::optimizer)
    }

    pub fn infer(&mut self, input: &[&Input]) -> Result<Vec<Output>, Failure> {
        if input.is_empty() {
            return Ok(Vec::new());
        }
        let batch = Batch::new(&self.device, input, self.model.configuration())?;
        let mut trace = self.scratch(batch.extent)?;
        let mut command = self.device.command()?;
        self.forward(&mut command, &batch, &trace, false)?;
        command.run()?;
        let output = read(&batch, &mut trace, self.model.configuration().judge);
        self.inference = Some(trace);
        Ok(output)
    }

    pub fn train(
        &mut self,
        sample: &[&Sample],
        weight: Weight,
        rate: f32,
    ) -> Result<(Loss, f32), Failure> {
        let mut adam = self.adam.take().ok_or(Failure::Unprepared)?;
        let outcome = self.descend(&mut adam, sample, weight, rate);
        self.adam = Some(adam);
        outcome
    }

    fn scratch(&mut self, extent: Extent) -> Result<Trace, Failure> {
        if let Some(trace) = self
            .inference
            .take()
            .filter(|trace| trace.extent.covers(extent))
        {
            return Ok(trace);
        }
        Trace::new(
            &self.device,
            self.model.configuration(),
            extent.widen(),
            false,
        )
    }

    fn workspace(&mut self, extent: Extent) -> Result<(Trace, Delta), Failure> {
        if let Some(workspace) = self
            .training
            .take()
            .filter(|(trace, _)| trace.extent.covers(extent))
        {
            return Ok(workspace);
        }
        let configuration = self.model.configuration();
        let extent = extent.widen();
        Ok((
            Trace::new(&self.device, configuration, extent, true)?,
            Delta::new(&self.device, configuration, extent)?,
        ))
    }

    fn descend(
        &mut self,
        adam: &mut Adam,
        sample: &[&Sample],
        weight: Weight,
        rate: f32,
    ) -> Result<(Loss, f32), Failure> {
        if sample.is_empty() {
            return Ok((Loss::default(), 0.0));
        }
        let input = sample
            .iter()
            .map(|sample| &sample.input)
            .collect::<Vec<_>>();
        let batch = Batch::new(&self.device, &input, self.model.configuration())?;
        let occurrence = Occurrence::new(
            &self.device,
            &batch.pack.feature,
            self.model.configuration(),
        )?;
        let (mut trace, mut delta) = self.workspace(batch.extent)?;
        let mut command = self.device.command()?;
        self.forward(&mut command, &batch, &trace, true)?;
        command.run()?;
        let output = read(&batch, &mut trace, self.model.configuration().judge);
        let (loss, change) = measure(&output, sample, weight);
        delta.load(&change);
        adam.advance(rate);
        let mut command = self.device.command()?;
        self.backward(
            &mut command,
            &batch,
            &occurrence,
            &trace,
            &delta,
            &adam.gradient,
        )?;
        self.update(&mut command, adam);
        command.run()?;
        adam.step += 1;
        self.training = Some((trace, delta));
        Ok((loss, adam.norm()))
    }

    fn linear<'memory>(
        &'memory self,
        command: &mut Command<'memory>,
        layer: Layer<'memory>,
    ) -> Result<(), Failure> {
        let weight = layer.linear.weight;
        dispatch(
            command,
            &self.pipeline.bias,
            &[layer.output, &self.parameter],
            &[
                layer.row,
                weight.column,
                layer.linear.bias.start,
                usize::from(layer.accumulate),
            ],
            [weight.column, layer.row, 1],
        );
        command.multiply(Product {
            left: operand(layer.input, 0, layer.row, weight.row, false),
            right: operand(
                &self.parameter,
                weight.start,
                weight.row,
                weight.column,
                false,
            ),
            result: operand(layer.output, 0, layer.row, weight.column, false),
            alpha: 1.0,
            beta: 1.0,
        })
    }

    fn project<'memory>(
        &'memory self,
        command: &mut Command<'memory>,
        input: &'memory Memory,
        row: usize,
        span: Span,
        output: &'memory Memory,
    ) -> Result<(), Failure> {
        command.multiply(Product {
            left: operand(input, 0, row, span.row, false),
            right: operand(&self.parameter, span.start, span.row, span.column, false),
            result: operand(output, 0, row, span.column, false),
            alpha: 1.0,
            beta: 0.0,
        })
    }

    fn norm<'memory>(
        &'memory self,
        command: &mut Command<'memory>,
        norm: &Norm,
        input: &'memory Memory,
        stage: &'memory Stage,
        row: usize,
        store: bool,
    ) {
        dispatch(
            command,
            &self.pipeline.norm,
            &[
                input,
                &self.parameter,
                &stage.output,
                &stage.normal,
                &stage.inverse,
            ],
            &[
                row,
                self.model.configuration().width,
                norm.scale.start,
                norm.shift.start,
                usize::from(store),
            ],
            [row, 1, 1],
        );
    }

    fn forward<'memory>(
        &'memory self,
        command: &mut Command<'memory>,
        batch: &'memory Batch,
        trace: &'memory Trace,
        store: bool,
    ) -> Result<(), Failure> {
        let configuration = self.model.configuration();
        let (token, sample) = (batch.extent.token, batch.extent.sample);
        let (width, hidden) = (configuration.width, configuration.hidden);
        dispatch(
            command,
            &self.pipeline.embed,
            &[&batch.feature, &self.parameter, &self.table, &trace.state],
            &[token, width, configuration.field.len()],
            [width, token, 1],
        );
        for (index, block) in self.model.block().iter().enumerate() {
            let record = if store {
                &trace.record[index]
            } else {
                &trace.record[0]
            };
            self.norm(
                command,
                &block.first,
                &trace.state,
                &record.first,
                token,
                store,
            );
            self.linear(
                command,
                Layer {
                    input: &record.first.output,
                    row: token,
                    linear: &block.attention.project,
                    output: &record.mixed,
                    accumulate: false,
                },
            )?;
            tiled(
                command,
                &self.pipeline.attend,
                &[
                    &record.mixed,
                    &record.context,
                    &batch.partition,
                    &record.statistic,
                ],
                &[width, configuration.head, usize::from(store)],
                [batch.tile, configuration.head],
            );
            self.linear(
                command,
                Layer {
                    input: &record.context,
                    row: token,
                    linear: &block.attention.output,
                    output: &trace.state,
                    accumulate: true,
                },
            )?;
            self.norm(
                command,
                &block.second,
                &trace.state,
                &record.second,
                token,
                store,
            );
            self.linear(
                command,
                Layer {
                    input: &record.second.output,
                    row: token,
                    linear: &block.feed.expand,
                    output: &record.activation,
                    accumulate: false,
                },
            )?;
            dispatch(
                command,
                &self.pipeline.activate,
                &[&record.activation, &record.hidden],
                &[token * hidden],
                [token * hidden, 1, 1],
            );
            self.linear(
                command,
                Layer {
                    input: &record.hidden,
                    row: token,
                    linear: &block.feed.contract,
                    output: &trace.state,
                    accumulate: true,
                },
            )?;
        }
        self.norm(
            command,
            self.model.norm(),
            &trace.state,
            &trace.last,
            token,
            store,
        );
        let head = self.model.head();
        dispatch(
            command,
            &self.pipeline.gather,
            &[&trace.last.output, &batch.summary, &trace.summary],
            &[sample, width],
            [width, sample, 1],
        );
        self.linear(
            command,
            Layer {
                input: &trace.summary,
                row: sample,
                linear: &head.value,
                output: &trace.before,
                accumulate: false,
            },
        )?;
        dispatch(
            command,
            &self.pipeline.activate,
            &[&trace.before, &trace.after],
            &[sample * width],
            [sample * width, 1, 1],
        );
        self.linear(
            command,
            Layer {
                input: &trace.after,
                row: sample,
                linear: &head.score,
                output: &trace.score,
                accumulate: false,
            },
        )?;
        self.linear(
            command,
            Layer {
                input: &trace.after,
                row: sample,
                linear: &head.judge,
                output: &trace.judge,
                accumulate: false,
            },
        )?;
        self.linear(
            command,
            Layer {
                input: &trace.last.output,
                row: token,
                linear: &head.unary,
                output: &trace.choice,
                accumulate: false,
            },
        )?;
        self.project(command, &trace.last.output, token, head.query, &trace.query)?;
        self.project(command, &trace.last.output, token, head.key, &trace.key)?;
        dispatch(
            command,
            &self.pipeline.point,
            &[
                &batch.pointer,
                &trace.choice,
                &trace.query,
                &trace.key,
                &trace.logit,
            ],
            &[
                batch.extent.pointer,
                configuration.unary,
                configuration.key,
                configuration.binary * configuration.key,
            ],
            [batch.extent.pointer, 1, 1],
        );
        Ok(())
    }

    fn clear<'memory>(
        &self,
        command: &mut Command<'memory>,
        memory: &'memory Memory,
        count: usize,
    ) {
        dispatch(
            command,
            &self.pipeline.clear,
            &[memory],
            &[count],
            [count, 1, 1],
        );
    }

    fn weight<'memory>(
        &'memory self,
        command: &mut Command<'memory>,
        gradient: &'memory Memory,
        input: &'memory Memory,
        change: &'memory Memory,
        row: usize,
        linear: &Linear,
    ) -> Result<(), Failure> {
        let span = linear.weight;
        command.multiply(Product {
            left: operand(input, 0, row, span.row, true),
            right: operand(change, 0, row, span.column, false),
            result: operand(gradient, span.start, span.row, span.column, false),
            alpha: 1.0,
            beta: 1.0,
        })?;
        dispatch(
            command,
            &self.pipeline.column,
            &[change, gradient],
            &[row, span.column, linear.bias.start, BLOCK],
            [span.column, row.div_ceil(BLOCK), 1],
        );
        Ok(())
    }

    fn back<'memory>(
        &'memory self,
        command: &mut Command<'memory>,
        change: &'memory Memory,
        row: usize,
        span: Span,
        output: &'memory Memory,
        accumulate: bool,
    ) -> Result<(), Failure> {
        command.multiply(Product {
            left: operand(change, 0, row, span.column, false),
            right: operand(&self.parameter, span.start, span.row, span.column, true),
            result: operand(output, 0, row, span.row, false),
            alpha: 1.0,
            beta: if accumulate { 1.0 } else { 0.0 },
        })
    }

    fn unnorm<'memory>(
        &'memory self,
        command: &mut Command<'memory>,
        norm: &Norm,
        stage: &'memory Stage,
        delta: &'memory Delta,
        gradient: &'memory Memory,
        row: usize,
    ) {
        let width = self.model.configuration().width;
        dispatch(
            command,
            &self.pipeline.renorm,
            &[&delta.normal, &stage.normal, gradient],
            &[row, width, norm.scale.start, norm.shift.start, BLOCK],
            [width, row.div_ceil(BLOCK), 1],
        );
        dispatch(
            command,
            &self.pipeline.unnorm,
            &[
                &delta.normal,
                &stage.normal,
                &stage.inverse,
                &self.parameter,
                &delta.state,
            ],
            &[row, width, norm.scale.start, 1],
            [row, 1, 1],
        );
    }

    fn backward<'memory>(
        &'memory self,
        command: &mut Command<'memory>,
        batch: &'memory Batch,
        occurrence: &'memory Occurrence,
        trace: &'memory Trace,
        delta: &'memory Delta,
        gradient: &'memory Memory,
    ) -> Result<(), Failure> {
        let configuration = self.model.configuration();
        let (token, sample) = (batch.extent.token, batch.extent.sample);
        let (width, hidden) = (configuration.width, configuration.hidden);
        let stride = configuration.binary * configuration.key;
        let head = self.model.head();
        self.clear(command, gradient, self.model.size());
        self.clear(command, &delta.choice, token * configuration.unary);
        self.clear(command, &delta.query, token * stride);
        self.clear(command, &delta.key, token * stride);
        self.clear(command, &delta.normal, token * width);
        self.clear(command, &delta.state, token * width);
        dispatch(
            command,
            &self.pipeline.unpoint,
            &[
                &batch.pointer,
                &delta.logit,
                &trace.query,
                &trace.key,
                &delta.choice,
                &delta.query,
                &delta.key,
            ],
            &[
                batch.extent.pointer,
                configuration.unary,
                configuration.key,
                stride,
            ],
            [batch.extent.pointer, 1, 1],
        );
        self.weight(
            command,
            gradient,
            &trace.after,
            &delta.value,
            sample,
            &head.score,
        )?;
        self.back(
            command,
            &delta.value,
            sample,
            head.score.weight,
            &delta.before,
            false,
        )?;
        self.weight(
            command,
            gradient,
            &trace.after,
            &delta.judge,
            sample,
            &head.judge,
        )?;
        self.back(
            command,
            &delta.judge,
            sample,
            head.judge.weight,
            &delta.before,
            true,
        )?;
        dispatch(
            command,
            &self.pipeline.derive,
            &[&delta.before, &trace.before],
            &[sample * width],
            [sample * width, 1, 1],
        );
        self.weight(
            command,
            gradient,
            &trace.summary,
            &delta.before,
            sample,
            &head.value,
        )?;
        self.back(
            command,
            &delta.before,
            sample,
            head.value.weight,
            &delta.summary,
            false,
        )?;
        dispatch(
            command,
            &self.pipeline.spread,
            &[&delta.summary, &batch.summary, &delta.normal],
            &[sample, width],
            [width, sample, 1],
        );
        self.weight(
            command,
            gradient,
            &trace.last.output,
            &delta.choice,
            token,
            &head.unary,
        )?;
        self.back(
            command,
            &delta.choice,
            token,
            head.unary.weight,
            &delta.normal,
            true,
        )?;
        for (span, change) in [(head.query, &delta.query), (head.key, &delta.key)] {
            command.multiply(Product {
                left: operand(&trace.last.output, 0, token, width, true),
                right: operand(change, 0, token, span.column, false),
                result: operand(gradient, span.start, span.row, span.column, false),
                alpha: 1.0,
                beta: 1.0,
            })?;
            self.back(command, change, token, span, &delta.normal, true)?;
        }
        self.unnorm(
            command,
            self.model.norm(),
            &trace.last,
            delta,
            gradient,
            token,
        );
        for (block, record) in self.model.block().iter().zip(&trace.record).rev() {
            self.weight(
                command,
                gradient,
                &record.hidden,
                &delta.state,
                token,
                &block.feed.contract,
            )?;
            self.back(
                command,
                &delta.state,
                token,
                block.feed.contract.weight,
                &delta.hidden,
                false,
            )?;
            dispatch(
                command,
                &self.pipeline.derive,
                &[&delta.hidden, &record.activation],
                &[token * hidden],
                [token * hidden, 1, 1],
            );
            self.weight(
                command,
                gradient,
                &record.second.output,
                &delta.hidden,
                token,
                &block.feed.expand,
            )?;
            self.back(
                command,
                &delta.hidden,
                token,
                block.feed.expand.weight,
                &delta.normal,
                false,
            )?;
            self.unnorm(
                command,
                &block.second,
                &record.second,
                delta,
                gradient,
                token,
            );
            self.weight(
                command,
                gradient,
                &record.context,
                &delta.state,
                token,
                &block.attention.output,
            )?;
            self.back(
                command,
                &delta.state,
                token,
                block.attention.output.weight,
                &delta.context,
                false,
            )?;
            tiled(
                command,
                &self.pipeline.unattend,
                &[
                    &record.mixed,
                    &record.context,
                    &delta.context,
                    &batch.partition,
                    &record.statistic,
                    &delta.inner,
                    &delta.mixed,
                ],
                &[width, configuration.head],
                [batch.tile, configuration.head],
            );
            tiled(
                command,
                &self.pipeline.release,
                &[
                    &record.mixed,
                    &delta.context,
                    &batch.partition,
                    &record.statistic,
                    &delta.inner,
                    &delta.mixed,
                ],
                &[width, configuration.head],
                [batch.tile, configuration.head],
            );
            self.weight(
                command,
                gradient,
                &record.first.output,
                &delta.mixed,
                token,
                &block.attention.project,
            )?;
            self.back(
                command,
                &delta.mixed,
                token,
                block.attention.project.weight,
                &delta.normal,
                false,
            )?;
            self.unnorm(command, &block.first, &record.first, delta, gradient, token);
        }
        dispatch(
            command,
            &self.pipeline.unembed,
            &[
                &occurrence.order,
                &occurrence.offset,
                &self.target,
                &delta.state,
                gradient,
            ],
            &[width, occurrence.count],
            [width, occurrence.count, 1],
        );
        Ok(())
    }

    fn update<'memory>(&'memory self, command: &mut Command<'memory>, adam: &'memory Adam) {
        let size = self.model.size();
        self.clear(command, &adam.total, 1);
        dispatch(
            command,
            &self.pipeline.square,
            &[&adam.gradient, &adam.total],
            &[size],
            [size, 1, 1],
        );
        dispatch(
            command,
            &self.pipeline.adam,
            &[
                &self.parameter,
                &adam.gradient,
                &adam.moment,
                &adam.velocity,
                &adam.decay,
                &adam.total,
                &adam.constant,
            ],
            &[size],
            [size, 1, 1],
        );
    }
}
