use crate::application;
use crate::configuration::Configuration;
use crate::failure::Failure;
use crate::flow::{Flow, Place};
use crate::fragment::Fragment;
use crate::introduction;
use crate::structure::Value;
use crate::{context, occurrence, world};
use std::collections::{BTreeMap, BTreeSet};

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
    pub archive: BTreeMap<context::Identity, crate::archive::Origin>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Path {
    state: Vec<Configuration>,
    record: Vec<Record>,
    flow: Vec<Flow>,
}

impl Path {
    pub fn new(source: Configuration) -> Self {
        Self {
            flow: vec![Flow::identity(&source)],
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
        self.flow.last().unwrap()
    }

    pub(crate) fn prefix(&self, position: usize) -> Result<&Flow, Failure> {
        self.flow.get(position).ok_or(Failure::State(position))
    }

    pub fn advance(&self, step: Step) -> Result<Self, Failure> {
        let (target, flow, read, consumed, context, archive) = match &step {
            Step::Historical(request) => {
                let event = crate::admission::apply(self, request)?;
                (
                    event.target,
                    event.flow,
                    event.read,
                    event.consumed,
                    event.context,
                    event.archive,
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
                    BTreeMap::new(),
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
                    BTreeMap::new(),
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
                    event.archive,
                )
            }
        };
        flow.validate(self.target(), &target)
            .map_err(Failure::Flow)?;
        let composed = crate::transport::compose(self, &flow, &archive)?;
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
            archive,
        });
        let mut flow = self.flow.clone();
        flow.push(composed);
        Ok(Self {
            state,
            record,
            flow,
        })
    }
}
