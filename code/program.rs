use crate::rule::Rule;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(from = "Vec<Rule>", into = "Vec<Rule>")]
pub struct Program {
    rule: Vec<Rule>,
}

impl From<Vec<Rule>> for Program {
    fn from(mut rule: Vec<Rule>) -> Self {
        rule.sort_unstable();
        Self { rule }
    }
}

impl From<Program> for Vec<Rule> {
    fn from(program: Program) -> Self {
        program.rule
    }
}

impl Program {
    pub fn rule(&self) -> &[Rule] {
        &self.rule
    }

    pub fn flat(&self) -> bool {
        self.rule.iter().all(Rule::flat)
    }
}
