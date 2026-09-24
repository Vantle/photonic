use crate::seal::Seal;

pub trait Plain: Copy + Seal {}

impl Plain for f32 {}
impl Plain for u32 {}
impl Plain for i32 {}
