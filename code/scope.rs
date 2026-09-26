use crate::particle::Particle;
use crate::rule::Rule;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "Form", into = "Form")]
pub struct Scope {
    coherence: Vec<Particle>,
    rule: Vec<Rule>,
    scope: Vec<Self>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Form {
    coherence: Vec<Particle>,
    rule: Vec<Rule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    scope: Vec<Scope>,
}

impl TryFrom<Form> for Scope {
    type Error = &'static str;

    fn try_from(form: Form) -> Result<Self, Self::Error> {
        if form.rule.is_empty() {
            return Err("a scope lists a rule");
        }
        Ok(Self::new(form.coherence, form.rule, form.scope))
    }
}

impl From<Scope> for Form {
    fn from(scope: Scope) -> Self {
        Self {
            coherence: scope.coherence,
            rule: scope.rule,
            scope: scope.scope,
        }
    }
}

impl Scope {
    // A scope that lists neither a coherence nor a scope holds the empty coherence, as its text
    // does, so the remainder of the rule that opens a scope always has a coherence to enter.
    pub(crate) fn new(
        mut coherence: Vec<Particle>,
        mut rule: Vec<Rule>,
        mut scope: Vec<Self>,
    ) -> Self {
        if coherence.is_empty() && scope.is_empty() {
            coherence.push(Particle::default());
        }
        coherence.sort_unstable();
        rule.sort_unstable();
        scope.sort_unstable();
        Self {
            coherence,
            rule,
            scope,
        }
    }

    pub fn map(
        &self,
        particle: &mut impl FnMut(&Particle) -> Particle,
        rule: &mut impl FnMut(&Rule) -> Rule,
    ) -> Self {
        Self::new(
            self.coherence.iter().map(&mut *particle).collect(),
            self.rule.iter().map(&mut *rule).collect(),
            self.scope
                .iter()
                .map(|scope| scope.map(particle, rule))
                .collect(),
        )
    }

    pub fn coherence(&self) -> &[Particle] {
        &self.coherence
    }

    pub fn rule(&self) -> &[Rule] {
        &self.rule
    }

    pub fn scope(&self) -> &[Self] {
        &self.scope
    }
}
