use crate::failure::Failure;
use metal::device::{Device, Memory};
use metal::plain::Plain;
use std::marker::PhantomData;

// Device memory for elements of one type that grows by doubling.
pub struct Grow<Element> {
    pub memory: Memory,
    capacity: usize,
    element: PhantomData<Element>,
}

impl<Element: Plain> Grow<Element> {
    pub fn new(device: &Device, capacity: usize) -> Result<Self, Failure> {
        let capacity = capacity.max(16);
        Ok(Self {
            memory: device.space::<Element>(capacity)?,
            capacity,
            element: PhantomData,
        })
    }

    // Makes room for needed elements, losing what the memory held.
    pub fn fit(&mut self, device: &Device, needed: usize) -> Result<(), Failure> {
        if needed <= self.capacity {
            return Ok(());
        }
        let capacity = needed.max(2 * self.capacity);
        self.memory = device.space::<Element>(capacity)?;
        self.capacity = capacity;
        Ok(())
    }

    // Makes room for needed elements in fresh memory and hands back the memory it replaces, whose
    // elements a command copies over before anything reads them.
    pub fn swap(&mut self, device: &Device, needed: usize) -> Result<Option<Memory>, Failure> {
        if needed <= self.capacity {
            return Ok(None);
        }
        let capacity = needed.max(2 * self.capacity);
        let memory = device.space::<Element>(capacity)?;
        self.capacity = capacity;
        Ok(Some(std::mem::replace(&mut self.memory, memory)))
    }
}
