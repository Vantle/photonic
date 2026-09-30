use crate::rule;
use frontend::source::Program;
use photonic::snapshot::Definition;
use serde::Serialize;
use spectrum::order::Naming;
use std::collections::HashMap;

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
    // An exploration runs the canonical program, whose rules hold canonical letters in their order.
    // Each rule reads as the source writes it, which is how the symmetry analysis names rules too;
    // a rule the source never writes out reads in the source's names.
    pub fn new(definition: &[Definition], program: &Program, naming: &Naming) -> Self {
        let hidden = naming.hide(program);
        let mut written = HashMap::new();
        for (canonical, source) in rule::program(&hidden)
            .into_iter()
            .zip(rule::program(program))
        {
            written
                .entry(canonical.canonical())
                .or_insert_with(|| frontend::text::definition(source));
        }
        Self {
            name: definition
                .iter()
                .map(|entry| {
                    written
                        .get(&entry.rule.canonical())
                        .cloned()
                        .unwrap_or_else(|| frontend::text::definition(&naming.show(&entry.rule)))
                })
                .collect(),
        }
    }

    pub fn name(&self, rule: usize) -> &str {
        &self.name[rule]
    }
}
