use super::{Event, Report, Search};
use crate::render::{Builder, Sequence};
use crate::snapshot::Node;
use crate::status::Status;
use serde::Serialize;

impl Search {
    pub fn definition(&self) -> Vec<crate::snapshot::Definition> {
        Builder::new(&self.compiled).definition()
    }

    fn node(&self) -> impl Iterator<Item = Node> + '_ {
        let mut storage = crate::canonical::storage::Store::default();
        let mut builder = Builder::new(&self.compiled);
        self.state.iter().enumerate().map(move |(index, record)| {
            let canonical = record
                .canonical
                .get_or_init(|| storage.insert(record.state.canonical()));
            builder.node(index, &canonical.state, Status::Supported)
        })
    }

    fn event(&self) -> impl Iterator<Item = &Event> + '_ {
        (0..self.event.len()).map(|index| self.transition(index).unwrap())
    }

    pub fn stream(&self) -> impl Serialize + '_ {
        Report {
            definition: self.definition(),
            outcome: self.outcome(),
            witness: self.reached().then_some(self.cursor),
            work: self.work,
            program: &self.program,
            target: self.target.as_ref(),
            state: Sequence::new(|| self.node()),
            event: Sequence::new(|| self.event()),
        }
    }

    pub fn report(&self) -> Report {
        Report {
            definition: self.definition(),
            outcome: self.outcome(),
            witness: self.reached().then_some(self.cursor),
            work: self.work,
            program: self.program.clone(),
            target: self.target.clone(),
            state: self.node().collect(),
            event: self.event().cloned().collect(),
        }
    }
}
