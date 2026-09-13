use crate::failure::Failure;

#[derive(Clone, Copy)]
pub(crate) struct Format {
    base: u8,
    width: usize,
}

impl Format {
    pub(crate) fn new(base: u8, width: usize) -> Result<Self, Failure> {
        let maximum = match base {
            2 => 64,
            3 => 40,
            _ => {
                return Err(Failure::Base {
                    value: base.into(),
                    minimum: 2,
                    maximum: 3,
                });
            }
        };
        if !(1..=maximum).contains(&width) {
            return Err(Failure::Width {
                value: width,
                minimum: 1,
                maximum,
            });
        }
        Ok(Self { base, width })
    }

    pub(crate) fn operand(base: u8, width: usize) -> Result<Self, Failure> {
        let format = Self::new(base, width)?;
        let maximum = if base == 2 { 32 } else { 20 };
        if width > maximum {
            return Err(Failure::Width {
                value: width,
                minimum: 1,
                maximum,
            });
        }
        Ok(format)
    }

    pub(crate) fn base(self) -> u8 {
        self.base
    }
    pub(crate) fn width(self) -> usize {
        self.width
    }

    pub(crate) fn check(self, value: u64) -> Result<(), Failure> {
        if value as u128 >= (self.base as u128).pow(self.width as u32) {
            return Err(Failure::Capacity);
        }
        Ok(())
    }
}
