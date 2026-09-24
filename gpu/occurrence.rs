use crate::failure::Failure;
use crate::runtime::{Device, Memory};
use network::configuration::Configuration;

pub struct Occurrence {
    pub order: Memory,
    pub offset: Memory,
    pub count: usize,
}

impl Occurrence {
    pub fn new(
        device: &Device,
        feature: &[u16],
        configuration: &Configuration,
    ) -> Result<Self, Failure> {
        let field = configuration.field.len();
        let base = configuration
            .field
            .iter()
            .scan(0, |total, &cardinality| {
                let start = *total;
                *total += cardinality;
                Some(start)
            })
            .collect::<Vec<_>>();
        let count = configuration.field.iter().sum::<usize>();
        let value = feature
            .iter()
            .enumerate()
            .map(|(index, &feature)| base[index % field] + usize::from(feature))
            .collect::<Vec<_>>();
        let mut frequency = vec![0u32; count];
        for &value in &value {
            frequency[value] += 1;
        }
        let offset = std::iter::once(0)
            .chain(frequency.iter().scan(0, |total, &frequency| {
                *total += frequency;
                Some(*total)
            }))
            .collect::<Vec<u32>>();
        let mut cursor = offset.clone();
        let mut order = vec![0u32; value.len()];
        for (index, &value) in value.iter().enumerate() {
            order[cursor[value] as usize] = (index / field) as u32;
            cursor[value] += 1;
        }
        Ok(Self {
            order: device.upload(&order)?,
            offset: device.upload(&offset)?,
            count,
        })
    }
}
