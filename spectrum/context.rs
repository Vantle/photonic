use crate::budget::Budget;
use crate::exploration::{Exploration, Plan};
use crate::explored::Explored;
use crate::failure::{Code, Failure};
use crate::recording::{Engine, Mode, Recording};
use crate::store::Store;
use crate::subject::Reader;
use frontend::source::Program;
use std::sync::Arc;

pub struct Context<'context> {
    pub reader: &'context dyn Reader,
    pub store: &'context mut Store,
}

impl Context<'_> {
    // An exploration that records every event, which every question but explore and check needs; a
    // recording that asks for metal is refused before anything is explored.
    pub(crate) fn exploration(
        &mut self,
        recording: &Recording,
    ) -> Result<Arc<Exploration>, Failure> {
        let refused = || {
            Failure::new(
                Code::Engine,
                "metal keeps only counts, ends and cycles, no events, so it answers explore and check alone; ask this with laser",
            )
        };
        if recording.engine == Some(Engine::Metal) {
            return Err(refused());
        }
        match self.explored(recording)? {
            Explored::Exploration(exploration) => Ok(exploration),
            Explored::Survey(_) => Err(refused()),
        }
    }

    pub(crate) fn explored(&mut self, recording: &Recording) -> Result<Explored, Failure> {
        match (&recording.program, &recording.exploration) {
            (Some(program), None) => {
                let source = program.assemble(self.reader)?;
                self.explore(&source, recording)
            }
            (None, Some(key)) => {
                if recording.mode.is_some()
                    || recording.engine.is_some()
                    || recording.budget.is_some()
                    || recording.goal.is_some()
                {
                    return Err(Failure::new(
                        Code::Request,
                        "an exploration key fixes the mode, engine, budget and goal; give them with program instead",
                    ));
                }
                self.store.find(key)
            }
            (Some(_), Some(_)) => Err(Failure::new(
                Code::Request,
                "give program or exploration, not both",
            )),
            (None, None) => Err(Failure::new(
                Code::Request,
                "give the program, or the exploration key an earlier answer returned",
            )),
        }
    }

    fn explore(&mut self, source: &Program, recording: &Recording) -> Result<Explored, Failure> {
        let mode = recording.mode.unwrap_or_default();
        let engine = match (mode, recording.engine) {
            (Mode::Plain, Some(Engine::Interpreter)) => {
                return Err(Failure::new(
                    Code::Request,
                    "plain mode runs on laser or metal; the interpreter explores with inference",
                ));
            }
            (_, Some(Engine::Metal)) if mode != Mode::Plain => {
                return Err(Failure::new(
                    Code::Request,
                    "metal explores every schedule of plain events; set mode to plain",
                ));
            }
            (Mode::Path, Some(Engine::Laser)) => {
                return Err(Failure::new(
                    Code::Request,
                    "a direct path follows the interpreter's scheduler; laser explores every future, so leave engine out in path mode",
                ));
            }
            (Mode::Path, _) => Engine::Interpreter,
            (_, engine) => engine.unwrap_or_default(),
        };
        if mode != Mode::Path && recording.goal.is_some() {
            return Err(Failure::new(
                Code::Request,
                "goal is the configuration a direct path stops at; set mode to path",
            ));
        }
        let goal = recording
            .goal
            .as_ref()
            .map(|goal| {
                let mut target = crate::subject::lower("goal", &goal.configuration, Code::Target)?;
                if goal.preserve {
                    target.preserve(source);
                }
                Ok::<_, Failure>(target)
            })
            .transpose()?;
        let budget = recording.budget.unwrap_or_default();
        let keep = match engine {
            Engine::Metal => crate::survey::capacity()?,
            Engine::Interpreter | Engine::Laser => budget.limit().configuration,
        };
        self.store.explore(Plan::new(
            source,
            mode,
            engine,
            Budget {
                configuration: Some(budget.configuration.unwrap_or(keep)),
                ..budget
            },
            goal,
        ))
    }
}
