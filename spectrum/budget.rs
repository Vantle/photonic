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
        Self {
            work: 2_000_000,
            configuration: 4_096,
            coherence: 64,
            occurrence: 256,
            scope: 64,
            record: 2_000_000,
        }
    }
}

impl Budget {
    pub(crate) fn limit(self) -> Limit {
        Limit {
            state: self.configuration,
            record: self.record,
            world: self.coherence,
            cell: self.occurrence,
            frame: self.scope,
        }
    }
}
