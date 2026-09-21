use crate::failure::Failure;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Identity(pub u64);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Branch(pub u64);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct History {
    choice: BTreeMap<Identity, Branch>,
}

impl History {
    pub fn decide(&self, identity: Identity, branch: Branch) -> Result<Self, Failure> {
        if let Some(&actual) = self.choice.get(&identity) {
            if actual != branch {
                return Err(Failure::History {
                    identity,
                    expected: branch,
                    actual: Some(actual),
                });
            }
            return Ok(self.clone());
        }
        let mut result = self.clone();
        result.choice.insert(identity, branch);
        Ok(result)
    }

    pub fn permits(&self, source: &Self) -> Result<(), Failure> {
        for (&identity, &expected) in &source.choice {
            let actual = self.choice.get(&identity).copied();
            if actual != Some(expected) {
                return Err(Failure::History {
                    identity,
                    expected,
                    actual,
                });
            }
        }
        Ok(())
    }
}
