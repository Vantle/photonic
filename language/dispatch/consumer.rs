use super::entry::Consumer;
use super::{Key, Network};
use crate::index::Index;
use crate::mask::Set;
use crate::profile;
use crate::reader::Read;

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
struct Priority {
    depth: usize,
    resource: usize,
}

pub(super) struct Request {
    pub key: Key,
    priority: Priority,
    pub consumer: Consumer,
}

impl Network {
    pub(super) fn request(
        &mut self,
        index: &Index,
        frame: usize,
        selected: Option<&Set>,
        request: &mut Vec<Request>,
    ) {
        let _scope = profile::Scope::new(profile::Phase::Request);
        if self.enabled.is_empty() || !index.present(frame) {
            return;
        }
        let ancestry =
            std::iter::successors(Some(frame), |&frame| index.state.frame[frame].lexical)
                .collect::<smallvec::SmallVec<[usize; 4]>>();
        for input in selected
            .unwrap_or(&self.enabled)
            .iter()
            .filter(|&input| self.enabled.contains(input))
        {
            let plan = self.catalog.input(input);
            for (depth, &current) in ancestry.iter().enumerate() {
                if plan.arity() == 0 && frame != current {
                    continue;
                }
                self.membership.ensure(index, &self.catalog, current);
                request.extend(
                    self.membership
                        .select(&self.catalog, index, current, input)
                        .map(|consumer| Request {
                            key: Key {
                                frame,
                                input,
                                owner: plan.owner(consumer.owner),
                            },
                            priority: Priority {
                                depth,
                                resource: match consumer.read {
                                    Some(Read::Context(_, resource)) => resource,
                                    _ => 0,
                                },
                            },
                            consumer,
                        }),
                );
            }
        }
        for reader in index.reader(frame) {
            let input = self.catalog.rule(reader.rule);
            if !self.enabled.contains(input)
                || selected.is_some_and(|selected| !selected.contains(input))
            {
                continue;
            }
            request.push(Request {
                key: Key {
                    frame,
                    input,
                    owner: self.catalog.input(input).owner(reader.owner),
                },
                priority: Priority {
                    depth: usize::MAX,
                    resource: 0,
                },
                consumer: Consumer {
                    rule: reader.rule,
                    owner: reader.owner,
                    read: Some(reader.read()),
                },
            });
        }
        request.sort_by_key(|request| (request.key, request.priority));
    }
}
