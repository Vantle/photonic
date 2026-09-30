use crate::recording::Mode;
use crate::render;
use schemars::JsonSchema;
use serde::Serialize;

// How far an exploration's answers reach, which every answer about one carries: every future, or
// every plain schedule, once it closed; what it explored so far while it is open; or the one run a
// direct path follows. Answers word what it has not settled by it.
#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Extent {
    pub(crate) mode: Mode,
    #[schemars(
        description = "Whether the exploration closed: it explored every future, or in plain mode every plain schedule, within its budget, so what it lacks is absent. A direct path follows one run and never closes."
    )]
    pub(crate) closed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    Closed,
    Open,
    Path,
}

impl Extent {
    pub(crate) fn kind(self) -> Kind {
        match (self.mode, self.closed) {
            (Mode::Path, _) => Kind::Path,
            (Mode::Exhaustive | Mode::Plain, true) => Kind::Closed,
            (Mode::Exhaustive | Mode::Plain, false) => Kind::Open,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self.kind() {
            Kind::Closed => "closed",
            Kind::Open => "open",
            Kind::Path => "a direct path",
        }
    }

    // That a rule did not fire, in words that claim no more than the exploration settled.
    pub(crate) fn never(self) -> &'static str {
        match self.kind() {
            Kind::Closed => "never",
            Kind::Open => "not yet",
            Kind::Path => "not on this path",
        }
    }

    // How often a rule fired, in words that claim no more than the exploration settled.
    pub(crate) fn firing(self, fired: usize) -> String {
        let time = render::count(fired, "time");
        match (self.kind(), fired) {
            (Kind::Closed, 0) => "never fires".to_owned(),
            (Kind::Open, 0) => "has not fired yet".to_owned(),
            (Kind::Path, 0) => "does not fire on this path".to_owned(),
            (Kind::Closed, _) => format!("fires {time}"),
            (Kind::Open, _) => format!("has fired {time} so far"),
            (Kind::Path, _) => format!("fires {time} on this path"),
        }
    }

    // That the events listed at a configuration may not be all that can happen there.
    pub(crate) fn partial(self) -> Option<&'static str> {
        match self.kind() {
            Kind::Closed => None,
            Kind::Open => Some("the exploration is open; more events may happen here"),
            Kind::Path => {
                Some("a direct path records only the event it took; more may happen here")
            }
        }
    }
}
