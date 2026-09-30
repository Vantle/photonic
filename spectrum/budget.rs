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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Configurations kept: photonic_test's 4,096 by default, and for metal as many as the GPU holds, half the memory Metal recommends at 256 bytes a configuration, or 1,048,576 on the host."
    )]
    pub configuration: Option<usize>,
    #[schemars(description = "Coherences in one configuration.")]
    pub coherence: usize,
    #[schemars(description = "Occurrences in one configuration.")]
    pub occurrence: usize,
    #[schemars(description = "Scopes in one configuration.")]
    pub scope: usize,
    #[schemars(
        description = "Records the engine retains; metal retains none, so it ignores the record budget and leaves it out of its key."
    )]
    pub record: usize,
}

impl Default for Budget {
    fn default() -> Self {
        let limit = Limit::default();
        Self {
            work: 2_000_000,
            configuration: None,
            coherence: limit.coherence,
            occurrence: limit.occurrence,
            scope: limit.scope,
            record: limit.record,
        }
    }
}

impl Budget {
    // The budget metal explores under: as many configurations as its device keeps unless told, and
    // the default record budget, since metal retains no records and the record budget must not
    // tell two of its explorations apart.
    pub(crate) fn metal(self, capacity: usize) -> Self {
        Self {
            configuration: Some(self.configuration.unwrap_or(capacity)),
            record: Self::default().record,
            ..self
        }
    }

    // The limits an engine explores under, keeping photonic_test's configurations unless told.
    pub fn limit(self) -> Limit {
        Limit {
            configuration: self.configuration.unwrap_or(Limit::default().configuration),
            record: self.record,
            coherence: self.coherence,
            occurrence: self.occurrence,
            scope: self.scope,
        }
    }
}
