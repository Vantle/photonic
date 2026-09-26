use crate::rule::Rule;
use crate::scope::Scope;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(from = "Form", into = "Form")]
pub struct Program {
    rule: Vec<Rule>,
    scope: Vec<Scope>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Form {
    rule: Vec<Rule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    scope: Vec<Scope>,
}

impl From<Form> for Program {
    fn from(form: Form) -> Self {
        Self::new(form.rule, form.scope)
    }
}

impl From<Program> for Form {
    fn from(program: Program) -> Self {
        Self {
            rule: program.rule,
            scope: program.scope,
        }
    }
}

impl From<Vec<Rule>> for Program {
    fn from(rule: Vec<Rule>) -> Self {
        Self::new(rule, Vec::new())
    }
}

impl Program {
    pub fn new(mut rule: Vec<Rule>, mut scope: Vec<Scope>) -> Self {
        rule.sort_unstable();
        scope.sort_unstable();
        Self { rule, scope }
    }

    pub fn rule(&self) -> &[Rule] {
        &self.rule
    }

    pub fn scope(&self) -> &[Scope] {
        &self.scope
    }

    pub fn flat(&self) -> bool {
        self.scope.is_empty() && self.rule.iter().all(Rule::flat)
    }
}
