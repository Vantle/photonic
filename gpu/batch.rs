use crate::failure::Failure;
use crate::pipeline::TILE;
use crate::runtime::{Device, Memory};
use network::configuration::Configuration;
use network::input::{Input, Pointer};
use network::pack::Pack;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Extent {
    pub token: usize,
    pub sample: usize,
    pub pointer: usize,
}

impl Extent {
    pub fn covers(&self, other: Self) -> bool {
        self.token >= other.token && self.sample >= other.sample && self.pointer >= other.pointer
    }

    pub fn widen(&self) -> Self {
        let widen = |value: usize| value + value / 4;
        Self {
            token: widen(self.token),
            sample: widen(self.sample),
            pointer: widen(self.pointer),
        }
    }
}

pub struct Batch {
    pub extent: Extent,
    pub pack: Pack,
    pub tile: usize,
    pub feature: Memory,
    pub partition: Memory,
    pub summary: Memory,
    pub pointer: Memory,
}

impl Batch {
    pub fn new(
        device: &Device,
        input: &[&Input],
        configuration: &Configuration,
    ) -> Result<Self, Failure> {
        let pack = Pack::new(input, configuration.field.len());
        let extent = Extent {
            token: pack.feature.len() / configuration.field.len(),
            sample: pack.boundary.len(),
            pointer: pack.pointer.iter().map(Vec::len).sum(),
        };
        let mut feature = device.memory::<u32>(pack.feature.len())?;
        for (target, value) in feature.edit::<u32>().iter_mut().zip(&pack.feature) {
            *target = u32::from(*value);
        }
        let partition = pack
            .boundary
            .iter()
            .flat_map(|range| {
                (0..range.len())
                    .step_by(TILE)
                    .flat_map(|first| [range.start as u32, range.len() as u32, first as u32])
            })
            .collect::<Vec<_>>();
        let summary = pack
            .boundary
            .iter()
            .map(|range| range.start as u32)
            .collect::<Vec<_>>();
        let mut pointer = device.memory::<u32>(4 * extent.pointer)?;
        for (target, value) in pointer
            .edit::<u32>()
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(pack.pointer.iter().flatten())
        {
            *target = match *value {
                Pointer::Unary { head, token } => [0, u32::from(head), token, 0],
                Pointer::Binary { head, left, right } => [1, u32::from(head), left, right],
            };
        }
        Ok(Self {
            extent,
            pack,
            tile: partition.len() / 3,
            feature,
            partition: device.upload(&partition)?,
            summary: device.upload(&summary)?,
            pointer,
        })
    }
}
