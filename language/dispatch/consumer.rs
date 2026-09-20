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
        let mut request = Vec::new();
        if index.present(frame) {
            let mut owner = Some(frame);
            while let Some(current) = owner {
                let scope = index.state.frame[current].scope;
                let available = &self.scope[scope];
                let (candidate, filter) = match selected {
                    Some(selected) if selected.len() < available.len() => {
                        (selected, Some(available))
                    }
                    selected => (available, selected),
                };
                for &input in candidate {
                    if filter.is_some_and(|filter| !filter.contains(&input)) {
                        continue;
                    }
                    let plan = self.catalog.input(input);
                    for &rule in self.catalog.scope(scope, input) {
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
