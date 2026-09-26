use crate::failure::{Defect, Failure};
use crate::pipeline::TILE;
use crate::runtime::{Device, Memory};
use network::configuration::Configuration;
use network::input::{Input, Pointer};
use network::pack::Pack;

fn inspect(input: &Input, configuration: &Configuration) -> Result<(), Defect> {
    let field = configuration.field.len();
    let feature = input.feature.len();
    if field == 0 || !feature.is_multiple_of(field) {
        return Err(Defect::Length {
            length: feature,
            field,
        });
    }
    if feature == 0 {
        return Err(Defect::Empty);
    }
    for (index, &value) in input.feature.iter().enumerate() {
        let cardinality = configuration.field[index % field];
        if usize::from(value) >= cardinality {
            return Err(Defect::Value {
                token: index / field,
                field: index % field,
                value,
                cardinality,
            });
        }
    }
    let length = input.length(field);
    for pointer in &input.pointer {
        let (head, count, target) = match *pointer {
            Pointer::Unary { head, token } => (head, configuration.unary, [token, token]),
            Pointer::Binary { head, left, right } => (head, configuration.binary, [left, right]),
        };
        if usize::from(head) >= count {
            return Err(Defect::Head { head, count });
        }
        if let Some(token) = target.into_iter().find(|&token| token as usize >= length) {
            return Err(Defect::Token { token, length });
        }
    }
    Ok(())
}

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
        for (sample, input) in input.iter().enumerate() {
            inspect(input, configuration).map_err(|defect| Failure::Input { sample, defect })?;
        }
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
