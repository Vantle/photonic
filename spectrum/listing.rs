use crate::budget::Budget;
use crate::exploration::{Exploration, Plan};
use crate::explore;
use crate::explored::Explored;
use crate::failure::{Code, Failure};
use crate::handle::Handle;
use crate::recording::{Engine, Mode};
use crate::render;
use frontend::source::Program;
use photonic::prism::{Outcome, Verdict};
use std::sync::Arc;

// Every configuration an exploration reaches, for the engine's own run and prism commands, which list
// a whole exploration instead of asking a question of it. They explore as a question's recording
// does, so the same program, mode, engine and budget give the key and handles every answer gives,
// and they write configurations and speak of explorations in the words answers use.
pub struct Listing {
    exploration: Arc<Exploration>,
    summary: String,
}

impl Listing {
    // A direct path follows the interpreter's scheduler to its goal, and plain mode runs on Laser;
    // metal keeps no configurations to list.
    pub fn new(
        program: &Program,
        mode: Mode,
        engine: Engine,
        budget: Budget,
        goal: Option<Program>,
    ) -> Result<Self, Failure> {
        let engine = match (mode, engine) {
            (_, Engine::Metal) => {
                return Err(Failure::new(
                    Code::Engine,
                    "metal keeps only counts, ends and cycles, so it lists no configurations; list them with laser",
                ));
            }
            (Mode::Plain, Engine::Interpreter) => {
                return Err(Failure::new(
                    Code::Request,
                    "plain mode runs on laser; the interpreter explores with inference",
                ));
            }
            (Mode::Path, _) => Engine::Interpreter,
            (Mode::Exhaustive | Mode::Plain, engine) => engine,
        };
        if mode != Mode::Path && goal.is_some() {
            return Err(Failure::new(
                Code::Request,
                "a goal is the configuration a direct path stops at",
            ));
        }
        // A question's recording writes the configurations it keeps into its budget, and so into its
        // key; writing them here too gives a listing the key that question gives.
        let budget = Budget {
            configuration: Some(budget.limit().configuration),
            ..budget
        };
        let exploration = Arc::new(Exploration::new(Plan::new(
            program, mode, engine, budget, goal,
        )));
        let summary = explore::state(&explore::brief(&Explored::Exploration(Arc::clone(
            &exploration,
        ))));
        Ok(Self {
            exploration,
            summary,
        })
    }

    // The exploration's summary, then each configuration by its handle.
    pub fn text(&self) -> String {
        let count = self.exploration.configuration.len();
        let width = Handle::Configuration(count.saturating_sub(1))
            .to_string()
            .len()
            .max(5);
        let line = (0..count).map(|index| {
            let note = if self.exploration.configuration[index].supported {
                ""
            } else {
                "   unsupported"
            };
            format!(
                "{:<width$} {}{note}",
                Handle::Configuration(index).to_string(),
                render::configuration(&self.exploration, index)
            )
        });
        std::iter::once(self.summary.clone())
            .chain(line)
            .collect::<Vec<_>>()
            .join("\n")
    }

    // Prism's verdict for an exact target, its witness numbered as every answer numbers
    // configurations. A direct path is a search for its goal, which is the target, so the path's own
    // outcome answers: reached where it stopped, or unknown.
    pub fn verdict(&self, target: &Program) -> Verdict {
        self.exploration
            .verdict(target, false)
            .unwrap_or_else(|| Verdict {
                outcome: if self.exploration.reached {
                    Outcome::Reached
                } else {
                    Outcome::Unknown
                },
                witness: self.exploration.reached.then(|| {
                    self.exploration
                        .event
                        .last()
                        .map_or(0, |event| event.target)
                }),
            })
    }

    // A verdict with its evidence, then the exploration's summary: the witness with the events that
    // reach it, or why no configuration is the target.
    pub fn judge(&self, verdict: &Verdict) -> String {
        let line = match verdict.outcome {
            Outcome::Reached => format!(
                "reached   {}",
                verdict
                    .witness
                    .map_or_else(String::new, |witness| self.witness(witness))
            ),
            Outcome::Unreachable => {
                "unreachable   no configuration is the target, and the exploration closed".to_owned()
            }
            Outcome::Unknown if self.exploration.mode == Mode::Path => {
                "unknown   a direct path follows one run of many, so it cannot show the target unreachable".to_owned()
            }
            Outcome::Unknown => {
                "unknown   the exploration stopped at its budget before it found the target"
                    .to_owned()
            }
        };
        format!("{line}\n{}", self.summary)
    }

    fn witness(&self, index: usize) -> String {
        let text = render::configuration(&self.exploration, index);
        let path = self.exploration.path(index).unwrap_or_default();
        if path.is_empty() {
            return format!("{}   {text}", Handle::Configuration(index));
        }
        let event = path
            .iter()
            .map(|&event| Handle::Event(event).to_string())
            .collect::<Vec<_>>();
        format!(
            "{} by {}   {text}",
            Handle::Configuration(index),
            event.join(" ")
        )
    }
}
