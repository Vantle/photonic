use super::{Application, Cache, Consumer, Query, Request, Runtime, Task};
use crate::flow::Place;
use crate::plan;
use crate::program::Symbol;
use crate::slot::Slot;
use std::sync::Arc;
use std::task::Poll;

impl Runtime {
    fn wake(&mut self, request: usize) {
        if self.active.insert(request) {
            self.agenda.push_back(Task::Deliver(request));
        }
    }

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
        let cache = if let Some(&cache) = self.matching.get(&key) {
            cache
        } else {
            let cache = self.cache.len();
            let index = self.index(key.target);
            let search = crate::search::Search::new(key.pattern.clone(), index, key.frame);
            let viable = search.viable();
            let retained = if viable { search.retained() } else { 0 };
            self.retained += retained;
            self.cache.push(Cache {
                search: viable.then_some(search),
                binding: Vec::new(),
                listener: Vec::new(),
                retained,
            });
            self.matching.insert(key, cache);
            if viable {
                self.agenda.push_back(Task::Search(cache));
            }
            cache
        };
        if self.cache[cache].search.is_none() && self.cache[cache].binding.is_empty() {
            return;
        }
        let (index, fresh) = self.request.insert_full(Request { cache, consumer });
        if !fresh {
            return;
        }
        self.cursor.push(0);
        self.cache[cache].listener.push(index);
        if !self.cache[cache].binding.is_empty() {
            self.wake(index);
        }
    }

    pub(super) fn search(&mut self, cache: usize, progress: Poll<Option<Vec<Slot>>>) {
        let retained = self.cache[cache].search.as_ref().unwrap().retained();
        self.retained = self.retained - self.cache[cache].retained + retained;
        self.cache[cache].retained = retained;
        match progress {
            Poll::Ready(None) => {
                self.retained -= self.cache[cache].retained;
                self.cache[cache].retained = 0;
                self.cache[cache].search = None;
                return;
            }
            Poll::Ready(Some(binding)) => {
                self.binding += 1;
                self.cache[cache].binding.push(Arc::new(binding));
                for index in self.cache[cache].listener.clone() {
                    self.wake(index);
                }
            }
            Poll::Pending => {}
        }
        self.agenda.push_back(Task::Search(cache));
    }

    pub(super) fn deliver(&mut self, index: usize) {
        self.active.remove(&index);
        let request = self.request[index].clone();
        let cache = &self.cache[request.cache];
        let Some(selection) = cache.binding.get(self.cursor[index]).cloned() else {
            return;
        };
        self.cursor[index] += 1;
        if self.cursor[index] < cache.binding.len() {
            self.wake(index);
        }
        let consumer = request.consumer;
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
