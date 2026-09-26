use crate::exploration::{Exploration, Plan};
use crate::failure::{Code, Failure};
use crate::recording::{Mode, Recording};
use crate::store::Store;
use crate::subject::Reader;
use frontend::source::Program;
use std::sync::Arc;

pub struct Context<'context> {
    pub reader: &'context dyn Reader,
    pub store: &'context mut Store,
}

impl Context<'_> {
    pub(crate) fn exploration(
        &mut self,
        recording: &Recording,
    ) -> Result<Arc<Exploration>, Failure> {
        match (&recording.program, &recording.exploration) {
            (Some(program), None) => {
                let source = program.assemble(self.reader)?;
                self.explore(&source, recording)
            }
            (None, Some(key)) => {
                if recording.mode.is_some()
                    || recording.budget.is_some()
                    || recording.goal.is_some()
                {
                    return Err(Failure::new(
                        Code::Request,
                        "an exploration key fixes the mode, budget and goal; give them with program instead",
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

    pub(crate) fn explore(
        &mut self,
        source: &Program,
        recording: &Recording,
    ) -> Result<Arc<Exploration>, Failure> {
        let mode = recording.mode.unwrap_or_default();
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
                let mut target = crate::subject::lower("goal", &goal.configuration, Code::Target)
                    .and_then(crate::subject::target)?;
                if goal.preserve {
                    target.preserve(source);
                }
                Ok::<_, Failure>(target)
            })
            .transpose()?;
        Ok(self.store.explore(Plan::new(
            source,
            mode,
            recording.budget.unwrap_or_default(),
            goal,
        )))
    }
}
