use crate::failure::Failure;
use crate::operand::Product;
use crate::plain::Plain;
use std::marker::PhantomData;

enum Never {}

pub struct Device {
    never: Never,
}

impl Device {
    pub fn open() -> Result<Self, Failure> {
        Err(Failure::Unavailable(
            "Metal requires macOS on Apple hardware".to_owned(),
        ))
    }

    pub fn name(&self) -> &str {
        match self.never {}
    }

    pub fn memory<Element: Plain>(&self, _: usize) -> Result<Memory, Failure> {
        match self.never {}
    }

    pub fn upload<Element: Plain>(&self, _: &[Element]) -> Result<Memory, Failure> {
        match self.never {}
    }

    pub fn library(&self, _: &str) -> Result<Library, Failure> {
        match self.never {}
    }

    pub fn kernel(&self, _: &Library, _: &str) -> Result<Kernel, Failure> {
        match self.never {}
    }

    pub fn command(&self) -> Result<Command<'_>, Failure> {
        match self.never {}
    }
}

pub struct Memory {
    never: Never,
}

impl Memory {
    pub fn view<Element: Plain>(&mut self) -> &[Element] {
        match self.never {}
    }

    pub fn edit<Element: Plain>(&mut self) -> &mut [Element] {
        match self.never {}
    }
}

pub enum Library {}

pub struct Kernel {
    never: Never,
}

impl Kernel {
    pub fn width(&self) -> usize {
        match self.never {}
    }

    pub fn capacity(&self) -> usize {
        match self.never {}
    }
}

pub struct Command<'device> {
    never: Never,
    borrow: PhantomData<&'device Memory>,
}

impl<'device> Command<'device> {
    pub fn dispatch(
        &mut self,
        _: &Kernel,
        _: &[&'device Memory],
        _: &[u8],
        _: [usize; 3],
        _: [usize; 3],
    ) {
        match self.never {}
    }

    pub fn multiply(&mut self, _: Product<'device>) -> Result<(), Failure> {
        match self.never {}
    }

    pub fn run(self) -> Result<(), Failure> {
        match self.never {}
    }
}
