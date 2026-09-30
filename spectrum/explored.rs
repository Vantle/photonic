use crate::exploration::{Exploration, Plan};
use crate::failure::Failure;
use crate::recording::Engine;
use crate::survey::Survey;
use std::sync::Arc;

// What exploring a recording found: an exploration that records every event, or, on metal, a
// survey of every plain schedule that keeps only counts, ends and cycles.
#[derive(Clone)]
pub(crate) enum Explored {
    Exploration(Arc<Exploration>),
    Survey(Arc<Survey>),
}

impl Explored {
    pub(crate) fn new(plan: Plan) -> Result<Self, Failure> {
        if plan.identity.engine == Engine::Metal {
            return Ok(Self::Survey(Arc::new(Survey::new(plan)?)));
        }
        Ok(Self::Exploration(Arc::new(Exploration::new(plan))))
    }

    pub(crate) fn key(&self) -> &str {
        match self {
            Self::Exploration(exploration) => &exploration.key,
            Self::Survey(survey) => &survey.key,
        }
    }

    // What an exploration weighs in the store: its occurrences and events, or a survey's ends.
    pub(crate) fn size(&self) -> usize {
        match self {
            Self::Exploration(exploration) => exploration.size(),
            Self::Survey(survey) => survey.end.len(),
        }
    }
}
