use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Pointer {
    Unary { head: u16, token: u32 },
    Binary { head: u16, left: u32, right: u32 },
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Input {
    pub feature: Vec<u16>,
    pub pointer: Vec<Pointer>,
}

impl Input {
    pub fn length(&self, field: usize) -> usize {
        self.feature.len() / field
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Output {
    pub logit: Vec<f32>,
    pub value: f32,
    pub judge: Vec<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Sample {
    pub input: Input,
    pub policy: Vec<f32>,
    pub value: Option<f32>,
    pub judge: Option<f32>,
}
