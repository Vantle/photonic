use crate::failure::Failure;
use crate::runtime::{Device, Memory};
use network::model::Model;
use network::optimizer::{Optimizer, Setting};

pub struct Adam {
    pub setting: Setting,
    pub step: u64,
    pub gradient: Memory,
    pub moment: Memory,
    pub velocity: Memory,
    pub decay: Memory,
    pub total: Memory,
    pub constant: Memory,
}

impl Adam {
    pub fn new(device: &Device, model: &Model, optimizer: &Optimizer) -> Result<Self, Failure> {
        let mut mask = vec![0u32; model.size()];
        for range in model.decay() {
            mask[range.clone()].fill(1);
        }
        Ok(Self {
            setting: optimizer.setting,
            step: optimizer.step,
            gradient: device.memory::<f32>(model.size())?,
            moment: device.upload(&optimizer.moment)?,
            velocity: device.upload(&optimizer.velocity)?,
            decay: device.upload(&mask)?,
            total: device.memory::<f32>(1)?,
            constant: device.memory::<f32>(8)?,
        })
    }

    pub fn advance(&mut self, rate: f32) {
        let step = i32::try_from(self.step + 1).unwrap_or(i32::MAX);
        let setting = self.setting;
        self.constant.edit::<f32>().copy_from_slice(&[
            rate,
            setting.decay,
            setting.first,
            setting.second,
            setting.epsilon,
            setting.clip,
            1.0 - setting.first.powi(step),
            1.0 - setting.second.powi(step),
        ]);
    }

    pub fn norm(&mut self) -> f32 {
        self.total.view::<f32>()[0].sqrt()
    }

    pub fn optimizer(&mut self) -> Optimizer {
        Optimizer {
            setting: self.setting,
            moment: self.moment.view::<f32>().to_vec(),
            velocity: self.velocity.view::<f32>().to_vec(),
            step: self.step,
        }
    }
}
