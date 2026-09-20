use super::table::{Consumer, Query};
use super::{Application, Runtime, Task};
use crate::flow::Place;
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
            self.agenda.push_back(Task::Search(index));
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
            self.agenda.push_back(Task::Search(index));
        }
    }

    pub(super) fn deliver(&mut self, index: usize) {
        let Some(delivery) = self.matching.deliver(index) else {
            return;
        };
        if delivery.again {
            self.agenda.push_back(Task::Deliver(index));
        }
        let consumer = delivery.consumer;
        let selection = delivery.selection;
        let view = self.view[consumer.view].clone();
        if let Some(Place::World(site, _)) = consumer.read
            && !selection.iter().any(|slot| slot.world == site)
        {
            return;
        }
        let selected = selection
            .iter()
            .map(|slot| (slot.world, slot.token.clone()))
            .collect::<Vec<_>>();
        let Some(mut binding) = view.flow.project(
            &self.state[view.source],
            &self.state[view.target],
            &selected,
            consumer.frame,
        ) else {
            return;
        };
        if let Some(read) = consumer.read {
            binding.read = view.flow.resource[&read].iter().copied().collect();
        }
        self.agenda.push_back(Task::Apply(Application {
            view: consumer.view,
            frame: consumer.frame,
            owner: consumer.owner,
            rule: consumer.rule,
            binding,
            capture: consumer.capture,
        }));
    }

    pub(super) fn inspect(&mut self, index: usize) {
        let view = self.view[index].clone();
        let source = self.state[view.source].clone();
        let target = self.state[view.target].clone();
        let available = self.index(view.target);
        let origin = self.index(view.source);
        let mut destination = vec![Vec::new(); source.frame.len()];
        for (frame, &source) in view.flow.frame.iter().enumerate() {
            if let Some(source) = source {
                destination[source].push(frame);
            }
        }
        for frame in origin.frame() {
            let mut owner = Some(frame);
            while let Some(current) = owner {
                let scope = source.frame[current].scope;
                let rule = self
                    .candidate
                    .entry((view.target, scope))
                    .or_insert_with(|| {
                        let candidate = self.program.scope[scope]
                            .candidate(available.available())
                            .into_iter()
                            .filter(|&rule| {
                                self.program.rule[rule]
                                    .input
                                    .iter()
                                    .flatten()
                                    .all(|symbol| available.contains(symbol))
                            })
                            .collect::<Vec<_>>();
                        self.indexed += candidate.len();
                        Arc::new(candidate)
                    })
                    .clone();
                let capture = destination[current].first().copied();
                for &rule in rule.iter() {
                    let input = if self.program.rule[rule].input.is_empty() {
                        vec![Vec::new()]
                    } else {
                        self.program.rule[rule].input.clone()
                    };
                    let pattern = plan::pattern(&input, capture);
                    for &destination in &destination[frame] {
                        self.subscribe(
                            Query {
                                target: view.target,
                                frame: destination,
                                pattern: pattern.clone(),
                            },
                            Consumer {
                                view: index,
                                frame,
                                owner: Some(current),
                                rule,
                                capture: None,
                                read: None,
                            },
                        );
                    }
                }
                owner = source.frame[current].lexical;
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
                let input = if self.program.rule[rule].input.is_empty() {
                    vec![Vec::new()]
                } else {
                    self.program.rule[rule].input.clone()
                };
                let pattern = plan::pattern(&input, token.capture);
                self.subscribe(
                    Query {
                        target: view.target,
                        frame: world.frame,
                        pattern,
                    },
                    Consumer {
                        view: index,
                        frame,
                        owner: token.capture.and_then(|capture| view.flow.frame[capture]),
                        rule,
                        capture: token.capture,
                        read: Some(Place::World(site, token.id)),
                    },
                );
            }
        }
    }
}
