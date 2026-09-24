use crate::particle::Particle;
use crate::rule::Rule;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(from = "Form", into = "Form")]
pub struct Output {
    particle: Particle,
    body: Option<Vec<Rule>>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Form {
    particle: Particle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    body: Option<Vec<Rule>>,
}

impl From<Form> for Output {
    fn from(form: Form) -> Self {
        Self::new(form.particle, form.body)
    }
}

impl From<Output> for Form {
    fn from(output: Output) -> Self {
        Self {
            particle: output.particle,
            body: output.body,
        }
    }
}

impl Output {
    pub fn new(particle: Particle, body: Option<Vec<Rule>>) -> Self {
        let body = body.filter(|rule| !rule.is_empty()).map(|mut rule| {
            rule.sort_unstable();
            rule
        });
        Self { particle, body }
    }

    pub fn plain(particle: Particle) -> Self {
        Self {
            particle,
            body: None,
        }
    }

    pub fn particle(&self) -> &Particle {
        &self.particle
    }

    pub fn body(&self) -> Option<&[Rule]> {
        self.body.as_deref()
    }
}
