use crate::failure::Failure;
use metal::device::{Device, Memory};
use metal::plain::Plain;
use std::marker::PhantomData;

// A host table on the device. A table the host only appends to has its new elements written alone,
// and one that outgrows its memory moves to at least twice as much and is written whole.
pub struct Mirror<Element> {
    pub memory: Memory,
    length: usize,
    element: PhantomData<Element>,
}

impl<Element: Plain> Mirror<Element> {
    pub fn new(device: &Device, value: &[Element]) -> Result<Self, Failure> {
        let mut mirror = Self {
            memory: device.space::<Element>(value.len())?,
            length: 0,
            element: PhantomData,
        };
        mirror.append(device, value)?;
        Ok(mirror)
    }

    pub fn append(&mut self, device: &Device, value: &[Element]) -> Result<(), Failure> {
        let capacity = self.memory.length() / std::mem::size_of::<Element>();
        if value.len() > capacity {
            self.memory = device.space::<Element>(value.len().max(2 * capacity))?;
            self.length = 0;
        }
        self.memory.edit::<Element>()[self.length..value.len()]
            .copy_from_slice(&value[self.length..]);
        self.length = value.len();
        Ok(())
    }

    // Writes a table that changed in place whole.
    pub fn rewrite(&mut self, device: &Device, value: &[Element]) -> Result<(), Failure> {
        self.length = 0;
        self.append(device, value)
    }
}
