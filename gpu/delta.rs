use crate::batch::Extent;
use crate::failure::Failure;
use crate::runtime::{Device, Memory};
use network::configuration::Configuration;

pub struct Delta {
    pub state: Memory,
    pub normal: Memory,
    pub mixed: Memory,
    pub context: Memory,
    pub hidden: Memory,
    pub inner: Memory,
    pub summary: Memory,
    pub before: Memory,
    pub value: Memory,
    pub judge: Memory,
    pub choice: Memory,
    pub query: Memory,
    pub key: Memory,
    pub logit: Memory,
}

impl Delta {
    pub fn new(
        device: &Device,
        configuration: &Configuration,
        extent: Extent,
    ) -> Result<Self, Failure> {
        let (token, sample, width) = (extent.token, extent.sample, configuration.width);
        let stride = configuration.binary * configuration.key;
        Ok(Self {
            state: device.memory::<f32>(token * width)?,
            normal: device.memory::<f32>(token * width)?,
            mixed: device.memory::<f32>(token * 3 * width)?,
            context: device.memory::<f32>(token * width)?,
            hidden: device.memory::<f32>(token * configuration.hidden)?,
            inner: device.memory::<f32>(token * configuration.head)?,
            summary: device.memory::<f32>(sample * width)?,
            before: device.memory::<f32>(sample * width)?,
            value: device.memory::<f32>(sample)?,
            judge: device.memory::<f32>(sample * configuration.judge)?,
            choice: device.memory::<f32>(token * configuration.unary)?,
            query: device.memory::<f32>(token * stride)?,
            key: device.memory::<f32>(token * stride)?,
            logit: device.memory::<f32>(extent.pointer)?,
        })
    }

    pub fn load(&mut self, change: &network::head::Delta) {
        self.value.edit::<f32>()[..change.value.len()].copy_from_slice(&change.value);
        for (target, value) in self
            .judge
            .edit::<f32>()
            .iter_mut()
            .zip(change.judge.iter().flatten())
        {
            *target = *value;
        }
        for (target, value) in self
            .logit
            .edit::<f32>()
            .iter_mut()
            .zip(change.logit.iter().flatten())
        {
            *target = *value;
        }
    }
}
