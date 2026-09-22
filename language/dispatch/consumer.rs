use super::entry::Consumer;
use super::{Key, Network};
use crate::index::Index;
use crate::membership::Set;

pub(super) struct Request {
    pub key: Key,
    pub consumer: Consumer,
}

impl Network {
    pub(super) fn request(
        &self,
        index: &Index,
        frame: usize,
        selected: Option<&Set>,
    ) -> Vec<Request> {
        #[cfg(feature = "measurement")]
        let _measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Request);
        let mut request = Vec::new();
        if self.enabled.is_empty() {
            return request;
        }
        if index.present(frame) {
            let mut owner = Some(frame);
            while let Some(current) = owner {
                let scope = index.state.frame[current].scope;
                let available = &self.scope[scope];
                let mut insert = |position| {
                    let (input, rule) = self.catalog.scope(scope, position);
                    let plan = self.catalog.input(input);
                    for &rule in rule {
                        request.push(Request {
                            key: Key {
                                frame,
                                input,
                                owner: plan.owner(current),
                            },
                            consumer: Consumer {
                                rule,
                                owner: current,
                                read: None,
                            },
                        });
                    }
                };
                if let Some(selected) = selected.filter(|selected| selected.len() < available.len())
                {
                    for &input in selected {
                        if let Some(position) = self.catalog.position(scope, input)
                            && available.contains(&position)
                        {
                            insert(position);
                        }
                    }
                } else {
                    for position in available.iter() {
                        let (input, _) = self.catalog.scope(scope, position);
                        if selected.is_none_or(|selected| selected.contains(&input)) {
                            insert(position);
                        }
                    }
                }
                owner = index.state.frame[current].lexical;
            }
            for reader in index.reader(frame) {
                let input = self.catalog.rule(reader.rule);
                if !self.enabled.contains(&input)
                    || selected.is_some_and(|selected| !selected.contains(&input))
                {
                    continue;
                }
                request.push(Request {
                    key: Key {
                        frame,
                        input,
                        owner: self.catalog.input(input).owner(reader.owner),
                    },
                    consumer: Consumer {
                        rule: reader.rule,
                        owner: reader.owner,
                        read: Some(reader.read),
                    },
                });
            }
        }
        request.sort_by_key(|request| request.key);
        request
    }
}
