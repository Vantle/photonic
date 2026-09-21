use crate::failure::Failure;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Allocation {
    pub occurrence: Option<u64>,
    pub context: Option<u64>,
    pub world: Option<u64>,
}

fn next(source: impl Iterator<Item = u64>) -> Option<u64> {
    source.max().map_or(Some(0), |value| value.checked_add(1))
}

pub(crate) fn take(value: &mut Option<u64>) -> Result<u64, Failure> {
    let result = value.ok_or(Failure::Capacity)?;
    *value = result.checked_add(1);
    Ok(result)
}

impl Allocation {
    pub fn new(
        occurrence: impl Iterator<Item = u64>,
        context: impl Iterator<Item = u64>,
        world: impl Iterator<Item = u64>,
    ) -> Self {
        Self {
            occurrence: next(occurrence),
            context: next(context),
            world: next(world),
        }
    }
}
