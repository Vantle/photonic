use crate::failure::{Code, Failure};
use photonic::snapshot::Definition;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
#[serde(transparent)]
pub struct Catalog {
    name: Vec<String>,
    #[serde(skip)]
    index: HashMap<String, usize>,
}

fn strip(display: &str) -> String {
    // The runtime displays a rule as a value, ⟨rule⟩, while events and the page name a rule by its text.
    display
        .strip_prefix('⟨')
        .and_then(|inner| inner.strip_suffix('⟩'))
        .unwrap_or(display)
        .to_owned()
}

impl From<Vec<Definition>> for Catalog {
    fn from(definition: Vec<Definition>) -> Self {
        Self {
            name: definition
                .iter()
                .map(|entry| strip(&entry.display))
                .collect(),
            index: definition
                .into_iter()
                .enumerate()
                .map(|(position, entry)| (entry.label, position))
                .collect(),
        }
    }
}

impl Catalog {
    pub fn rule(&self, label: &str) -> Result<usize, Failure> {
        self.index.get(label).copied().ok_or_else(|| {
            Failure::new(
                Code::Internal,
                format!("The runtime named the rule {label} without defining it."),
            )
        })
    }
}
