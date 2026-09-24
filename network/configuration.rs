use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub field: Vec<usize>,
    pub width: usize,
    pub depth: usize,
    pub head: usize,
    pub hidden: usize,
    pub unary: usize,
    pub binary: usize,
    pub key: usize,
    #[serde(default)]
    pub judge: usize,
}

impl Configuration {
    pub fn dimension(&self) -> usize {
        self.width / self.head
    }
}
