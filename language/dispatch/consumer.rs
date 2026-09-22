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
            for (place, token) in selected
                .unwrap_or(&self.enabled)
                .iter()
                .filter(|input| self.enabled.contains(input))
                .flat_map(|&input| self.catalog.member(input))
                .flat_map(|&rule| index.occurrence(frame, crate::program::Symbol::Rule(rule)))
            {
                let crate::program::Symbol::Rule(rule) = token.value else {
                    continue;
                };
                let crate::flow::Place::Context(current, resource) = place else {
                    unreachable!()
                };
                let input = self.catalog.rule(rule);
                let plan = self.catalog.input(input);
                if !self.enabled.contains(&input)
                    || selected.is_some_and(|selected| !selected.contains(&input))
                    || (plan.arity() == 0 && frame != current)
                {
                    continue;
                }
                let owner = token.capture.unwrap();
                request.push(Request {
                    key: Key {
                        frame,
                        input,
                        owner: plan.owner(owner),
                    },
                    consumer: Consumer {
                        rule,
                        owner,
                        read: Some(crate::reader::Read::Context(current, resource)),
                    },
                });
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
        let ancestry = std::iter::successors(index.present(frame).then_some(frame), |&frame| {
            index.state.frame[frame].lexical
        })
        .collect::<smallvec::SmallVec<[usize; 4]>>();
        request.sort_by_key(|request| {
            let priority = match request.consumer.read {
                Some(crate::reader::Read::Context(frame, resource)) => (
                    ancestry.iter().position(|&owner| owner == frame).unwrap(),
                    resource,
                ),
                _ => (usize::MAX, 0),
            };
            (request.key, priority)
        });
        request
    }
}
