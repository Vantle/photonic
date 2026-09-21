use crate::evidence::Evidence;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fragment<Value> {
    pub(crate) value: Value,
    pub(crate) evidence: Evidence,
}

impl<Value> Fragment<Value> {
    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }

    pub(crate) fn map<Target>(self, function: impl FnOnce(Value) -> Target) -> Fragment<Target> {
        Fragment {
            value: function(self.value),
            evidence: self.evidence,
        }
    }
}
