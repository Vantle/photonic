use photonic::runtime::Limit;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[serde(default)]
#[schemars(
    description = "Limits on exploration, named as in the book. Exhausting one leaves answers unknown; the defaults are photonic_test's."
)]
pub struct Budget {
    #[schemars(description = "Work steps before the search stops.")]
    pub work: usize,
    #[schemars(description = "Configurations kept.")]
    pub configuration: usize,
    #[schemars(description = "Coherences in one configuration.")]
    pub coherence: usize,
    #[schemars(description = "Occurrences in one configuration.")]
    pub occurrence: usize,
    #[schemars(description = "Scopes in one configuration.")]
    pub scope: usize,
    #[schemars(description = "Records the engine retains.")]
    pub record: usize,
}

impl Default for Budget {
    fn default() -> Self {
        let limit = Limit::default();
        Self {
            work: 2_000_000,
            configuration: limit.configuration,
            coherence: limit.coherence,
            occurrence: limit.occurrence,
            scope: limit.scope,
            record: limit.record,
        }
    }
}

impl Budget {
    pub fn limit(self) -> Limit {
        Limit {
            configuration: self.configuration,
            record: self.record,
            coherence: self.coherence,
            occurrence: self.occurrence,
            scope: self.scope,
        }
    }
}
