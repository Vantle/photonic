use crate::batch::Extent;
use crate::failure::Failure;
use crate::runtime::{Device, Memory};
use network::configuration::Configuration;

pub struct Stage {
    pub normal: Memory,
    pub inverse: Memory,
    pub output: Memory,
}

impl Stage {
    fn new(device: &Device, token: usize, width: usize, store: bool) -> Result<Self, Failure> {
        let kept = if store { token } else { 0 };
        Ok(Self {
            normal: device.memory::<f32>(kept * width)?,
            inverse: device.memory::<f32>(kept)?,
            output: device.memory::<f32>(token * width)?,
        })
    }
}

pub struct Record {
    pub first: Stage,
    pub mixed: Memory,
    pub statistic: Memory,
    pub context: Memory,
    pub second: Stage,
    pub activation: Memory,
    pub hidden: Memory,
}

impl Record {
    fn new(
        device: &Device,
        configuration: &Configuration,
        extent: Extent,
        store: bool,
    ) -> Result<Self, Failure> {
        let (token, width, hidden) = (extent.token, configuration.width, configuration.hidden);
        Ok(Self {
            first: Stage::new(device, token, width, store)?,
            mixed: device.memory::<f32>(token * 3 * width)?,
            statistic: device.memory::<f32>(if store { token * configuration.head } else { 0 })?,
            context: device.memory::<f32>(token * width)?,
            second: Stage::new(device, token, width, store)?,
            activation: device.memory::<f32>(token * hidden)?,
            hidden: device.memory::<f32>(token * hidden)?,
        })
    }
}

pub struct Trace {
    pub extent: Extent,
    pub state: Memory,
    pub record: Vec<Record>,
    pub last: Stage,
    pub summary: Memory,
    pub before: Memory,
    pub after: Memory,
    pub score: Memory,
    pub judge: Memory,
    pub choice: Memory,
    pub query: Memory,
    pub key: Memory,
    pub logit: Memory,
}

impl Trace {
    pub fn new(
        device: &Device,
        configuration: &Configuration,
        extent: Extent,
        store: bool,
    ) -> Result<Self, Failure> {
        let (token, sample, width) = (extent.token, extent.sample, configuration.width);
        let stride = configuration.binary * configuration.key;
        let layer = if store { configuration.depth } else { 1 };
        Ok(Self {
            extent,
            state: device.memory::<f32>(token * width)?,
            record: (0..layer)
                .map(|_| Record::new(device, configuration, extent, store))
                .collect::<Result<_, _>>()?,
            last: Stage::new(device, token, width, store)?,
            summary: device.memory::<f32>(sample * width)?,
            before: device.memory::<f32>(sample * width)?,
            after: device.memory::<f32>(sample * width)?,
            score: device.memory::<f32>(sample)?,
            judge: device.memory::<f32>(sample * configuration.judge)?,
            choice: device.memory::<f32>(token * configuration.unary)?,
            query: device.memory::<f32>(token * stride)?,
            key: device.memory::<f32>(token * stride)?,
            logit: device.memory::<f32>(extent.pointer)?,
        })
    }
}
