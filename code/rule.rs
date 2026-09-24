use crate::output::Output;
use crate::particle::Particle;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(from = "Form", into = "Form")]
pub struct Rule {
    input: Vec<Particle>,
    output: Vec<Output>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Form {
    input: Vec<Particle>,
    output: Vec<Output>,
}

impl From<Form> for Rule {
    fn from(form: Form) -> Self {
        Self::new(form.input, form.output)
    }
}

impl From<Rule> for Form {
    fn from(rule: Rule) -> Self {
        Self {
            input: rule.input,
            output: rule.output,
        }
    }
}

impl Rule {
    pub fn new(mut input: Vec<Particle>, mut output: Vec<Output>) -> Self {
        input.sort_unstable();
        output.sort_unstable();
        Self { input, output }
    }

    pub fn input(&self) -> &[Particle] {
        &self.input
    }

    pub fn output(&self) -> &[Output] {
        &self.output
    }

    pub fn flat(&self) -> bool {
        self.input
            .iter()
            .chain(self.output.iter().map(Output::particle))
            .all(|particle| particle.flat().is_some())
            && self.output.iter().all(|output| output.body().is_none())
    }
}
