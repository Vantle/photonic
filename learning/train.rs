use crate::play::Shared;
use gpu::engine::Engine;
use gpu::failure::Failure;
use network::input::Sample;
use network::loss::{Loss, Weight};
use network::model::Model;
use network::optimizer::Optimizer;
use random::Generator;
use rayon::ThreadPool;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct Setting {
    pub batch: usize,
    pub chunk: usize,
    pub rate: f32,
    pub warmup: u64,
    pub value: f32,
    pub judge: f32,
    pub publish: u64,
    pub ratio: f64,
    pub minimum: usize,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            batch: 128,
            chunk: 8,
            rate: 3e-4,
            warmup: 500,
            value: 1.0,
            judge: 1.0,
            publish: 20,
            ratio: 8.0,
            minimum: 1_024,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Progress {
    pub step: u64,
    pub policy: f64,
    pub value: f64,
    pub entropy: f64,
    pub norm: f64,
    pub judge: f64,
}

pub enum Device {
    Processor(ThreadPool),
    Graphics(Box<Engine>),
}

pub struct Trainer {
    model: Model,
    optimizer: Optimizer,
    device: Device,
    setting: Setting,
    generator: Generator,
}

fn descend(
    pool: &ThreadPool,
    model: &Model,
    sample: &[&Sample],
    chunk: usize,
    weight: Weight,
) -> (Vec<f32>, Loss) {
    let size = model.size();
    let chunk = sample.chunks(chunk.max(1)).collect::<Vec<_>>();
    pool.install(|| {
        chunk
            .par_iter()
            .map(|chunk| {
                let mut gradient = vec![0.0; size];
                let loss = model.gradient(chunk, &mut gradient, weight);
                (gradient, loss)
            })
            .reduce(
                || (vec![0.0; size], Loss::default()),
                |(mut left, total), (right, loss)| {
                    for (target, value) in left.iter_mut().zip(&right) {
                        *target += value;
                    }
                    (left, total.merge(loss))
                },
            )
    })
}

impl Trainer {
    pub fn new(
        model: Model,
        optimizer: Optimizer,
        mut device: Device,
        setting: Setting,
        seed: u64,
    ) -> Result<Self, Failure> {
        if let Device::Graphics(engine) = &mut device {
            engine.prepare(&optimizer)?;
        }
        Ok(Self {
            model,
            optimizer,
            device,
            setting,
            generator: Generator::new(seed),
        })
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn optimizer(&self) -> &Optimizer {
        &self.optimizer
    }

    fn rate(&self) -> f32 {
        let step = self.optimizer.step as f32;
        let warmup = self.setting.warmup.max(1) as f32;
        self.setting.rate * (step / warmup).min(1.0)
    }

    pub fn step(&mut self, batch: &[Arc<Sample>]) -> Result<Progress, Failure> {
        let weight = Weight {
            value: self.setting.value,
            judge: self.setting.judge,
            scale: 1.0 / batch.len().max(1) as f32,
        };
        let sample = batch.iter().map(AsRef::as_ref).collect::<Vec<_>>();
        let rate = self.rate();
        let (loss, norm) = match &mut self.device {
            Device::Processor(pool) => {
                let (gradient, loss) =
                    descend(pool, &self.model, &sample, self.setting.chunk, weight);
                let decay = self.model.decay().to_vec();
                let norm =
                    self.optimizer
                        .update(&mut self.model.parameter, &gradient, &decay, rate);
                (loss, norm)
            }
            Device::Graphics(engine) => {
                let outcome = engine.train(&sample, weight, rate)?;
                self.optimizer.step += 1;
                outcome
            }
        };
        let count = loss.count.max(1) as f64;
        Ok(Progress {
            step: self.optimizer.step,
            policy: loss.policy / count,
            value: loss.value / count,
            entropy: loss.entropy / count,
            norm: f64::from(norm),
            judge: if loss.judged == 0 {
                f64::NAN
            } else {
                loss.miss / loss.judged as f64
            },
        })
    }

    fn synchronize(&mut self) {
        let Device::Graphics(engine) = &mut self.device else {
            return;
        };
        self.model.parameter = engine.parameter();
        if let Some(optimizer) = engine.optimizer() {
            self.optimizer = optimizer;
        }
    }

    fn publish(&mut self, shared: &Shared) {
        self.synchronize();
        *shared
            .model
            .write()
            .expect("the model lock is never poisoned") = Arc::new(self.model.clone());
    }

    pub fn run(
        &mut self,
        shared: &Shared,
        progress: &Mutex<Progress>,
        keep: impl Fn(&Self),
    ) -> Result<(), Failure> {
        let mut saved = Instant::now();
        let origin = self.optimizer.step;
        while !shared.stop.load(Ordering::Relaxed) {
            let batch = {
                let replay = shared
                    .replay
                    .lock()
                    .expect("the replay lock is never poisoned");
                let consumed = (self.optimizer.step - origin) as f64 * self.setting.batch as f64;
                if replay.len() < self.setting.minimum
                    || consumed > self.setting.ratio * replay.total() as f64
                {
                    None
                } else {
                    Some(replay.draw(self.setting.batch, &mut self.generator))
                }
            };
            let Some(batch) = batch else {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            };
            let report = self.step(&batch)?;
            *progress
                .lock()
                .expect("the progress lock is never poisoned") = report;
            if report.step.is_multiple_of(self.setting.publish) {
                self.publish(shared);
            }
            if saved.elapsed() > Duration::from_secs(120) {
                self.synchronize();
                keep(self);
                saved = Instant::now();
            }
        }
        self.publish(shared);
        Ok(())
    }
}
