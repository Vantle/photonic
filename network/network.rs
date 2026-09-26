#![forbid(unsafe_code)]

pub mod attention;
pub mod block;
pub mod checkpoint;
pub mod configuration;
pub mod embedding;
pub mod feed;
pub mod grow;
pub mod head;
pub mod input;
mod layout;
pub mod linear;
pub mod loss;
pub mod matrix;
pub mod model;
pub mod norm;
pub mod optimizer;
pub mod pack;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;
