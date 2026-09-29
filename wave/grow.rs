use crate::failure::Failure;
use metal::device::{Device, Memory};
use metal::plain::Plain;
use std::marker::PhantomData;

// Device memory for elements of one type that grows by doubling.
pub struct Grow<Element> {
    pub memory: Memory,
    element: PhantomData<Element>,
}

impl<Element: Plain> Grow<Element> {
    pub fn new(device: &Device, capacity: usize) -> Result<Self, Failure> {
        Ok(Self {
            memory: device.space::<Element>(capacity.max(16))?,
            element: PhantomData,
        })
    }

    fn capacity(&self) -> usize {
        self.memory.length() / std::mem::size_of::<Element>()
    }

    // Makes room for needed elements, losing what the memory held.
    pub fn fit(&mut self, device: &Device, needed: usize) -> Result<(), Failure> {
        self.swap(device, needed).map(drop)
    }

    // Makes room for needed elements in fresh memory and hands back the memory it replaces, whose
    // elements a command copies over before anything reads them.
    pub fn swap(&mut self, device: &Device, needed: usize) -> Result<Option<Memory>, Failure> {
        if needed <= self.capacity() {
            return Ok(None);
        }
        let memory = device.space::<Element>(needed.max(2 * self.capacity()))?;
        Ok(Some(std::mem::replace(&mut self.memory, memory)))
    }
}
