use photonic::snapshot::Definition;
use serde::Serialize;

#[derive(Serialize)]
#[serde(transparent)]
pub struct Catalog {
    name: Vec<String>,
}

impl From<Vec<Definition>> for Catalog {
    fn from(definition: Vec<Definition>) -> Self {
        Self {
            name: definition.into_iter().map(|entry| entry.name).collect(),
        }
    }
}

impl Catalog {
    pub fn name(&self, rule: usize) -> &str {
        &self.name[rule]
    }
}
