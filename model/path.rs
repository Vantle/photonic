use crate::application;
use crate::configuration::Configuration;
use crate::failure::Failure;
use crate::flow::{Flow, Place};
use crate::fragment::Fragment;
use crate::introduction;
use crate::structure::Value;
use crate::{context, occurrence, world};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Step {
    Application(application::Request),
    Historical(crate::admission::Request),
    Inference {
        path: Box<Path>,
        request: application::Request,
    },
    Introduction {
        world: world::Identity,
        consumed: Vec<occurrence::Identity>,
        value: Fragment<Value>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    pub step: Step,
    pub flow: Flow,
    pub read: BTreeSet<Place>,
    pub consumed: BTreeSet<Place>,
    pub context: BTreeSet<context::Identity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Path {
    state: Vec<Configuration>,
    record: Vec<Record>,
    flow: Flow,
}

impl Path {
    pub fn new(source: Configuration) -> Self {
        Self {
            flow: Flow::identity(&source),
            state: vec![source],
            record: vec![],
        }
    }

    pub fn replay(
        source: Configuration,
        step: impl IntoIterator<Item = Step>,
    ) -> Result<Self, Failure> {
        let mut path = Self::new(source);
        for step in step {
            path = path.advance(step)?;
        }
        Ok(path)
    }

    pub fn source(&self) -> &Configuration {
        &self.state[0]
    }

    pub fn target(&self) -> &Configuration {
        self.state.last().unwrap()
    }

    pub fn state(&self) -> &[Configuration] {
        &self.state
    }

    pub fn record(&self) -> &[Record] {
        &self.record
    }

    pub fn flow(&self) -> &Flow {
        &self.flow
    }

    pub fn advance(&self, step: Step) -> Result<Self, Failure> {
        let (target, flow, read, consumed, context) = match &step {
            Step::Historical(request) => {
                let event = crate::admission::apply(self, request)?;
                (
                    event.target,
                    event.flow,
                    event.read,
                    event.consumed,
                    event.context,
                )
            }
            Step::Inference { path, request } => {
                if path.source() != self.target() {
                    return Err(Failure::Source);
                }
                let projection = crate::projection::project(path, request.clone())?;
                let event = projection.apply()?;
                (
                    event.target,
                    event.flow,
                    event.read,
                    event.consumed,
                    BTreeSet::from([event.owner]),
                )
            }
            Step::Application(request) => {
                let event = application::apply(self.target(), request)?;
                (
                    event.target,
                    event.flow,
                    event.read,
                    event.consumed,
                    BTreeSet::from([event.owner]),
                )
            }
            Step::Introduction {
                world,
                consumed,
                value,
            } => {
                let event = introduction::apply(self.target(), *world, consumed, value)?;
                (
                    event.target,
                    event.flow,
                    event.read,
                    event.consumed,
                    event.context,
                )
            }
        };
        flow.validate(self.target(), &target)
            .map_err(Failure::Flow)?;
        let composed = self.flow.compose(&flow).map_err(Failure::Flow)?;
        composed
            .validate(self.source(), &target)
            .map_err(Failure::Flow)?;
        let mut state = self.state.clone();
        state.push(target);
        let mut record = self.record.clone();
        record.push(Record {
            step,
            flow,
            read,
            consumed,
            context,
        });
        Ok(Self {
            state,
            record,
            flow: composed,
        })
    }
}
