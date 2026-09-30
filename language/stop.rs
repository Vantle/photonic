use schemars::JsonSchema;
use serde::Serialize;
use std::fmt::{self, Display, Formatter};

// What one configuration may hold at most, and how many configurations an exploration keeps.
#[derive(Clone, Copy, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
#[schemars(
    description = "A limit: the configurations an exploration keeps, or the coherences, occurrences or scopes one configuration holds."
)]
pub enum Bound {
    Configuration,
    Coherence,
    Occurrence,
    Scope,
}

// Every limit, in the order answers name them.
pub const BOUND: [Bound; 4] = [
    Bound::Configuration,
    Bound::Coherence,
    Bound::Occurrence,
    Bound::Scope,
];

// How many events each limit blocked.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Blocked([usize; 4]);

impl Blocked {
    pub fn add(&mut self, bound: Bound, count: usize) {
        self.0[bound as usize] = self.0[bound as usize].saturating_add(count);
    }

    pub fn get(self, bound: Bound) -> usize {
        self.0[bound as usize]
    }
}

impl FromIterator<Bound> for Blocked {
    fn from_iter<Iterator: IntoIterator<Item = Bound>>(bound: Iterator) -> Self {
        let mut blocked = Self::default();
        for bound in bound {
            blocked.add(bound, 1);
        }
        blocked
    }
}

// Why an exploration stopped: a direct path at its goal, back at a configuration it passed or where
// no event applies; or, short of closing, a budget that ran out, a limit that blocked events, and
// how many, or a start that already holds more than a limit allows.
#[derive(Clone, Copy, Debug, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Stop {
    #[schemars(description = "A direct path reached its goal.")]
    Reached,
    #[schemars(description = "A direct path came back to a configuration it passed.")]
    Cycle,
    #[schemars(description = "No event applies where a direct path stopped.")]
    End,
    #[schemars(description = "The work budget ran out with work left.")]
    Work { budget: usize },
    #[schemars(description = "The engine retained as many records as the record budget allows.")]
    Record { budget: usize },
    #[schemars(description = "A limit of this value blocked this many events.")]
    Limit {
        bound: Bound,
        value: usize,
        blocked: usize,
    },
    #[schemars(
        description = "The start already holds more than a limit of this value allows: measure coherences, occurrences or scopes."
    )]
    Start {
        bound: Bound,
        value: usize,
        measure: usize,
    },
}

impl Bound {
    // The budget flag and attribute that raise it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::Coherence => "coherence",
            Self::Occurrence => "occurrence",
            Self::Scope => "scope",
        }
    }
}

impl Stop {
    // The budget or limit to raise, named as its flag and attribute are.
    pub fn raise(self) -> Option<&'static str> {
        match self {
            Self::Reached | Self::Cycle | Self::End => None,
            Self::Work { .. } => Some("work"),
            Self::Record { .. } => Some("record"),
            Self::Limit { bound, .. } | Self::Start { bound, .. } => Some(bound.name()),
        }
    }
}

impl Display for Stop {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Reached => write!(formatter, "it reached its goal"),
            Self::Cycle => write!(formatter, "it came back to a configuration it passed"),
            Self::End => write!(formatter, "no event applies"),
            Self::Work { budget } => write!(formatter, "the work budget ({budget}) ran out"),
            Self::Record { budget } => write!(formatter, "the record budget ({budget}) filled"),
            Self::Limit {
                bound,
                value,
                blocked,
            } => write!(
                formatter,
                "the {} limit ({value}) blocked {blocked} {}",
                bound.name(),
                if blocked == 1 { "event" } else { "events" }
            ),
            Self::Start {
                bound,
                value,
                measure,
            } => write!(
                formatter,
                "s0 holds {measure} {}s; the {} limit is {value}",
                bound.name(),
                bound.name()
            ),
        }
    }
}

#[cfg(test)]
#[path = "test/stop.rs"]
mod test;
