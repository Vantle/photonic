use crate::runtime::Memory;

#[derive(Clone, Copy)]
pub struct Operand<'memory> {
    pub memory: &'memory Memory,
    pub offset: usize,
    pub row: usize,
    pub column: usize,
    pub transpose: bool,
}

#[derive(Clone, Copy)]
pub struct Product<'memory> {
    pub left: Operand<'memory>,
    pub right: Operand<'memory>,
    pub result: Operand<'memory>,
    pub alpha: f32,
    pub beta: f32,
}
