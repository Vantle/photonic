use super::table::{Consumer, Query};
use super::{Application, Runtime, Task};
use crate::application::Owner;
use crate::flow::Flow;
use crate::place::Place;
use crate::plan;
use crate::program::Symbol;
use crate::slot::Slot;
use std::sync::Arc;
use std::task::Poll;

impl Runtime {
    fn index(&mut self, state: usize) -> Arc<crate::index::Index> {
        self.index[state]
            .get_or_insert_with(|| {
                let index = Arc::new(crate::index::Index::new(self.state[state].clone()));
                self.indexed += index.retained();
                index
            })
            .clone()
    }

    fn subscribe(&mut self, key: Query, consumer: Consumer) {
        let index = self.index(key.target);
        let schedule = self.matching.subscribe(key, consumer, index);
        if let Some(index) = schedule.search {
            self.agenda.push(Task::Search(index));
        }
        self.agenda
            .extend(schedule.delivery.into_iter().map(Task::Deliver));
    }

    pub(super) fn search(
        &mut self,
        index: usize,
        search: crate::search::Search,
        progress: Poll<Option<Vec<Slot>>>,
    ) {
        let schedule = self.matching.advance(index, search, progress);
        self.agenda
            .extend(schedule.delivery.into_iter().map(Task::Deliver));
        if let Some(index) = schedule.search {
            self.agenda.push(Task::Search(index));
        }
    }

    pub(super) fn deliver(&mut self, index: usize) {
        let Some(delivery) = self.matching.deliver(index) else {
            return;
        };
        if delivery.again {
            self.agenda.push(Task::Deliver(index));
        }
        let consumer = delivery.consumer;
        let selection = delivery.selection;
        let view = self.view[consumer.view].clone();
        if let Some(Place::World(world, _)) = consumer.read
            && !crate::slot::admits(&selection, world)
        {
            return;
        }
        let Some(binding) = view.flow.project(
            &self.state[view.source],
            &self.state[view.target],
            &selection,
            consumer.frame,
            consumer.read,
        ) else {
            return;
        };
        self.agenda.push(Task::Apply(Application {
            view: consumer.view,
            frame: consumer.frame,
            owner: consumer.owner,
            rule: consumer.rule,
            binding,
        }));
    }

    pub(super) fn inspect(&mut self, index: usize) {
        let view = self.view[index].clone();
        let target = self.state[view.target].clone();
        let available = self.index(view.target);
        for frame in available.frame() {
            let Some(source) = view.flow.frame[frame] else {
                continue;
            };
            for (place, token) in target.visible(frame) {
                let Symbol::Rule(rule) = token.value else {
                    continue;
                };
                let Place::Context(owner, _) = place else {
                    unreachable!()
                };
                let input = &self.program.rule[rule].input;
                if (input.is_empty() && owner != frame)
                    || input
                        .iter()
                        .flatten()
                        .any(|symbol| !available.contains(symbol))
                {
                    continue;
                }
                let pattern = plan::pattern(input, token.capture);
                self.subscribe(
                    Query {
                        target: view.target,
                        frame,
                        pattern,
                    },
                    Consumer {
                        view: index,
                        frame: source,
                        owner: locate(token.capture, &view.flow),
                        rule,
                        read: Some(place),
                    },
                );
            }
        }
        for (site, world) in target.world.iter().enumerate() {
            let Some(frame) = view.flow.frame[world.frame] else {
                continue;
            };
            for token in &world.particle {
                let Symbol::Rule(rule) = token.value else {
                    continue;
                };
                if self.program.rule[rule]
                    .input
                    .iter()
                    .flatten()
                    .any(|symbol| !available.contains(symbol))
                {
                    continue;
                }
                let pattern = plan::pattern(&self.program.rule[rule].input, token.capture);
                self.subscribe(
                    Query {
                        target: view.target,
                        frame: world.frame,
                        pattern,
                    },
                    Consumer {
                        view: index,
                        frame,
                        owner: locate(token.capture, &view.flow),
                        rule,
                        read: Some(Place::World(site, token.id)),
                    },
                );
            }
        }
    }
}

fn locate(capture: Option<usize>, flow: &Flow) -> Owner<usize> {
    let capture = capture.expect("every rule token captures the frame it was made in");
    flow.frame[capture].map_or(Owner::Capture(capture), Owner::Frame)
}
