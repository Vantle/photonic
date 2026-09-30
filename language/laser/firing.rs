use super::focus::Focus;
use super::makeup::Makeup;
use super::taxonomy::Draft;
use super::trace::Trace;
use super::transition::{self, Effect, Key, Local};
use super::{Identity, Laser, Link, Round, map};
use crate::application::{self, Owner, Request};
use crate::executor::Executor;
use crate::flow::{Closure, Flow};
use crate::profile;
use crate::runtime::Measure;
use crate::state::Canonical;
use crate::stop::Bound;
use hashing::Builder;
use indexmap::IndexMap;
use std::ops::Range;
use std::sync::Arc;

// Applying an event holds its whole result until it is named, so a round applies and names its
// events a batch at a time; each batch finds the configurations and transitions the batches before
// it made, and events are numbered in the same order as with one batch.
const FIRING: usize = 1 << 14;

// An application is named in two steps: its components are named where it is made, and their
// numbers are given in a fixed order, so naming never depends on which worker applied first.
pub(super) struct Product {
    pub(super) draft: Draft,
    pub(super) flow: Flow,
    pub(super) whole: Option<Key>,
}

// An event over the parts it touches either finds its transition or applies itself to those parts
// alone, and what it learns serves every later event with the same key.
pub(super) struct Move {
    pub(super) local: Local,
    pub(super) known: Option<Arc<Effect>>,
    pub(super) product: Option<Product>,
}

pub(super) enum Outcome {
    Product(Box<Product>),
    Move(Box<Move>),
    Blocked(Bound),
}

enum Resolution {
    Absent,
    Known(usize),
    Blocked,
    Pending(usize),
}

struct Candidate {
    index: usize,
    pending: Vec<(Identity, usize)>,
    resolution: Vec<Resolution>,
}

impl Laser {
    pub(super) fn fire(
        &mut self,
        executor: Option<&Executor>,
        novel: Vec<(usize, Range<usize>)>,
        retry: Vec<(Identity, usize)>,
        next: &mut Round,
        allowance: usize,
    ) -> usize {
        // Identifying a trace holds its identity until it fires, so traces wait unidentified, and
        // identities blocked, while nothing can fire.
        if allowance == 0 || self.retained() >= self.limit.record {
            next.novel = novel;
            next.retry = retry;
            return 0;
        }
        let candidate = self.select(executor, novel);
        let revisit = retry
            .iter()
            .map(|(identity, position)| (identity.source, *position))
            .collect::<Vec<_>>();
        let mut pending = retry;
        let mut resolved = Vec::with_capacity(candidate.len());
        for value in candidate {
            resolved.push((value.index, pending.len(), value.resolution));
            pending.extend(value.pending);
        }
        let mut created = Vec::with_capacity(pending.len());
        let mut rest = pending.into_iter();
        loop {
            let room = FIRING.min(allowance - created.len());
            if room == 0 || self.retained() >= self.limit.record {
                break;
            }
            let batch = rest.by_ref().take(room).collect::<Vec<_>>();
            if batch.is_empty() {
                break;
            }
            let outcome = self.attempt(executor, &batch);
            created.extend(self.create(executor, batch, outcome, next));
            self.taxonomy.forget(executor);
        }
        // A budget spent within a round defers the identities its batches did not reach: they wait
        // as blocked identities do, so no trace fires one of them twice, and fire first next round.
        let count = created.len();
        for (identity, position) in rest {
            self.blocked.insert(identity.clone(), (position, None));
            next.retry.push((identity, position));
            created.push(None);
        }
        if !revisit.is_empty() {
            self.blocked
                .retain(|identity, _| !self.identity[identity.source].contains_key(identity));
        }
        for (index, offset, resolution) in resolved {
            self.link[index].extend(resolution.into_iter().map(|value| match value {
                Resolution::Absent => Link::Absent,
                Resolution::Known(event) => Link::Event(event),
                Resolution::Blocked => Link::Unresolved,
                Resolution::Pending(slot) => {
                    created[offset + slot].map_or(Link::Unresolved, Link::Event)
                }
            }));
        }
        for ((source, position), &event) in revisit.into_iter().zip(&created) {
            if let Some(event) = event {
                self.link[source][position] = Link::Event(event);
            }
        }
        count
    }

    fn select(
        &self,
        executor: Option<&Executor>,
        novel: Vec<(usize, Range<usize>)>,
    ) -> Vec<Candidate> {
        let _scope = profile::Scope::new(profile::Phase::Identification);
        map(executor, novel, |(index, range)| {
            self.candidate(index, range)
        })
    }

    fn candidate(&self, index: usize, range: Range<usize>) -> Candidate {
        let mut seen = IndexMap::<Identity, usize, Builder>::default();
        let mut resolution = Vec::with_capacity(range.len());
        let identity = range
            .clone()
            .map(|position| self.identify(index, &self.trace[index][position]))
            .collect::<Vec<_>>();
        let chosen = self.independence.as_ref().and_then(|independence| {
            let found = identity.iter().flatten().collect::<Vec<_>>();
            Focus::choose(&self.state[index], &found, independence)
        });
        for (position, identity) in range.zip(identity) {
            let Some(identity) = identity else {
                resolution.push(Resolution::Absent);
                continue;
            };
            if chosen.as_ref().is_some_and(|focus| !focus.holds(&identity)) {
                resolution.push(Resolution::Absent);
                continue;
            }
            if let Some(&event) = self.identity[index].get(&identity) {
                resolution.push(Resolution::Known(event));
                continue;
            }
            if self.blocked.contains_key(&identity) {
                resolution.push(Resolution::Blocked);
                continue;
            }
            let entry = seen.entry(identity);
            resolution.push(Resolution::Pending(entry.index()));
            entry.or_insert(position);
        }
        Candidate {
            index,
            pending: seen.into_iter().collect(),
            resolution,
        }
    }

    fn attempt(&self, executor: Option<&Executor>, pending: &[(Identity, usize)]) -> Vec<Outcome> {
        let _scope = profile::Scope::new(profile::Phase::Firing);
        map(
            executor,
            pending.iter().collect(),
            |(identity, position)| self.apply(identity, &self.trace[identity.source][*position]),
        )
    }

    // An event over parts applies to the parts it touches alone, once for every key, unless its
    // key made a configuration named whole; then, and for every other event, it applies to its
    // whole source.
    fn apply(&self, identity: &Identity, trace: &Trace) -> Outcome {
        let (Some(layout), Owner::Frame(owner)) = (&self.layout[identity.source], &identity.owner)
        else {
            return self.configuration(identity, trace, None);
        };
        let local = transition::localize(
            &self.taxonomy,
            layout,
            &self.makeup[identity.source],
            identity.frame,
            *owner,
            identity.rule,
            &identity.binding,
        );
        if let Some(known) = self.memo.get(&local.key) {
            return Outcome::Move(Box::new(Move {
                local,
                known: Some(known.clone()),
                product: None,
            }));
        }
        if self.whole.contains(&local.key) {
            return self.configuration(identity, trace, None);
        }
        let product = self.part(&local.key);
        if matches!(product.draft, Draft::Whole(_)) {
            return self.configuration(identity, trace, Some(local.key));
        }
        Outcome::Move(Box::new(Move {
            local,
            known: None,
            product: Some(product),
        }))
    }

    fn part(&self, key: &Key) -> Product {
        let makeup = Makeup {
            root: key.root,
            kind: key.kind.to_vec(),
        };
        let part = self.taxonomy.materialize(&makeup);
        let result = application::apply(Request {
            source: &part,
            scope: &self.program.scope,
            frame: key.frame,
            owner: Owner::Frame(key.owner),
            rule: &self.program.rule[key.rule],
            binding: &key.binding,
        });
        Product {
            draft: self.taxonomy.analyze(&result.state),
            flow: result.flow,
            whole: None,
        }
    }

    // A rule a detached capture owns applies through the flow back to the configuration where
    // its match was found.
    fn configuration(&self, identity: &Identity, trace: &Trace, whole: Option<Key>) -> Outcome {
        let attached = trace.owner().map(|capture| {
            let origin = &self.state[capture.origin];
            (capture, origin, capture.flow(origin, &self.pool))
        });
        let owner = match &attached {
            Owner::Frame(frame) => Owner::Frame(*frame),
            Owner::Capture((capture, origin, flow)) => Owner::Capture(Closure {
                state: origin,
                flow,
                capture: capture.frame,
            }),
        };
        let result = application::apply(Request {
            source: &self.state[identity.source],
            scope: &self.program.scope,
            frame: identity.frame,
            owner,
            rule: &self.program.rule[identity.rule],
            binding: &identity.binding,
        });
        if let Some(bound) = self.limit.refuse(Measure::new(&result.state)) {
            return Outcome::Blocked(bound);
        }
        Outcome::Product(Box::new(Product {
            draft: self.taxonomy.analyze(&result.state),
            flow: result.flow,
            whole,
        }))
    }

    fn canonical(&self, origin: usize, frame: usize) -> Arc<Canonical> {
        self.environment.get((origin, frame), |&(origin, frame)| {
            Arc::new(self.state[origin].environment(frame))
        })
    }

    fn identify(&self, source: usize, trace: &Trace) -> Option<Identity> {
        let binding = trace.binding(&self.state[source], &self.pool)?;
        let owner = trace.owner().map(|capture| {
            let canonical = self.canonical(capture.origin, capture.frame);
            Arc::new(capture.environment(&canonical))
        });
        Some(Identity {
            source,
            frame: trace.frame,
            owner,
            rule: trace.rule,
            binding,
        })
    }

    pub(super) fn identity(&self, event: usize) -> &Identity {
        let value = &self.event[event];
        self.identity[value.source]
            .get_index(value.slot)
            .expect("every event keeps its identity")
            .0
    }

    // The event a trace identifies: its link names it, or the trace forms its identity again when
    // the identity was blocked when the trace was found.
    pub(super) fn linked(&self, state: usize, position: usize) -> Option<usize> {
        match self.link[state].get(position)? {
            Link::Absent => None,
            Link::Event(event) => Some(*event),
            Link::Unresolved => {
                let identity = self.identify(state, &self.trace[state][position])?;
                self.identity[state].get(&identity).copied()
            }
        }
    }
}
