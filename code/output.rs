use crate::particle::Particle;
use crate::rule::Rule;
use crate::scope::Scope;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Output {
    Particle(Particle),
    Scope(Scope),
}

impl Output {
    // A group that lists no rule builds nothing, as in the text, so its members belong to the
    // enclosing list and every scope lists a rule.
    pub fn group(member: Vec<Self>, rule: Vec<Rule>) -> Vec<Self> {
        if rule.is_empty() {
            return member;
        }
        let mut coherence = Vec::new();
        let mut scope = Vec::new();
        for member in member {
            match member {
                Self::Particle(particle) => coherence.push(particle),
                Self::Scope(value) => scope.push(value),
            }
        }
        vec![Self::Scope(Scope::new(coherence, rule, scope))]
    }
}
