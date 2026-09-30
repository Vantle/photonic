use super::focus::Focus;
use super::makeup::Makeup;
use super::taxonomy::{Draft, Name};
use super::trace::Trace;
use super::transition::{self, Effect, Key, Local};
use super::{Identity, Laser, Link, Round, map};
use crate::application::{self, Owner, Request};
use crate::canonical::Exhausted;
use crate::executor::Executor;
use crate::flow::{Closure, Flow};
use crate::profile;
use crate::runtime::Measure;
use crate::state::{Canonical, State};
use crate::stop::Bound;
use hashing::Builder;
use indexmap::IndexMap;
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::Ordering;

// Applying an event holds its whole result until it is named, so a round applies and names its
// events a batch at a time, of at most FIRING events whose results hold about RESULT occurrences,
// live rules included, as many as the configurations so far held on average; each batch finds the
// configurations and transitions the batches before it made, and events are numbered in the same
// order as with one batch.
const FIRING: usize = 1 << 14;
const RESULT: usize = 1 << 22;

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

// An application the limits refuse is blocked until they change; one whose result takes more steps
// to name than the round allows is deferred to the next round, as the rest of a spent round is.
pub(super) enum Outcome {
    Product(Box<Product>),
    Move(Box<Move>),
    Blocked(Bound),
    Deferred,
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

// The configurations whose novel traces a round identified, those left for the next round, and
// the steps naming their owners' environments took.
struct Selection {
    candidate: Vec<Candidate>,
    rest: Vec<(usize, Range<usize>)>,
    charge: usize,
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
        let selection = self.select(executor, novel, allowance);
        next.novel = selection.rest;
        let revisit = retry
            .iter()
            .map(|(identity, position)| (identity.source, *position))
            .collect::<Vec<_>>();
        let mut pending = retry;
        let mut resolved = Vec::with_capacity(selection.candidate.len());
        for value in selection.candidate {
            resolved.push((value.index, pending.len(), value.resolution));
            pending.extend(value.pending);
        }
        let mut created = Vec::with_capacity(pending.len());
        let mut rest = pending.into_iter();
        let average = self.bulk.div_ceil(self.state.len()).max(1);
        let size = FIRING.min(RESULT / average).max(1);
        let mut used = selection.charge;
        loop {
            let room = size.min(allowance - used);
            if room == 0 || self.retained() >= self.limit.record {
                break;
            }
            let batch = rest.by_ref().take(room).collect::<Vec<_>>();
            if batch.is_empty() {
                break;
            }
            let outcome = self.attempt(executor, &batch, allowance - used - 1);
            let outcome = spend(outcome, &mut used, allowance);
            created.extend(self.create(executor, batch, outcome, next));
            self.taxonomy.forget(executor);
        }
        // A budget spent within a round defers the identities its batches did not reach: they wait
        // as blocked identities do, so no trace fires one of them twice, and fire first next round.
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
        used
    }

    // Identifies the novel traces in turn, naming the environments of those a capture owns with
    // what the allowance leaves after the traces before them. The first trace whose names do not
    // fit waits for the next round with every trace after it, and spends what was left, as a
    // search that runs out does; a reduced exploration chooses among a configuration's events all
    // at once, so there the whole configuration waits. Workers identify configurations at once,
    // each with the whole allowance, and the configuration that does not fit is identified again
    // afterwards to find the trace that does not, so what a round charges never depends on how
    // many workers identified it.
    fn select(
        &self,
        executor: Option<&Executor>,
        novel: Vec<(usize, Range<usize>)>,
        allowance: usize,
    ) -> Selection {
        let _scope = profile::Scope::new(profile::Phase::Identification);
        let identified = map(executor, novel, |(index, range)| {
            let mut budget = allowance;
            let identity = range
                .clone()
                .map(|position| self.identify(index, &self.trace[index][position], &mut budget))
                .collect::<Result<Vec<_>, Exhausted>>();
            let found = identity.map(|identity| {
                (
                    self.candidate(index, range.clone(), identity),
                    allowance - budget,
                )
            });
            (index, range, found)
        });
        let mut selection = Selection {
            candidate: Vec::with_capacity(identified.len()),
            rest: Vec::new(),
            charge: 0,
        };
        for (index, range, found) in identified {
            if !selection.rest.is_empty() {
                selection.rest.push((index, range));
                continue;
            }
            match found {
                Ok((candidate, cost)) if selection.charge + cost <= allowance => {
                    selection.charge += cost;
                    selection.candidate.push(candidate);
                }
                _ => {
                    let mut budget = allowance - selection.charge;
                    let mut identity = Vec::new();
                    if self.independence.is_none() {
                        for position in range.clone() {
                            let trace = &self.trace[index][position];
                            let Ok(found) = self.identify(index, trace, &mut budget) else {
                                break;
                            };
                            identity.push(found);
                        }
                    }
                    let split = range.start + identity.len();
                    if !identity.is_empty() {
                        selection.candidate.push(self.candidate(
                            index,
                            range.start..split,
                            identity,
                        ));
                    }
                    selection.charge = allowance;
                    selection.rest.push((index, split..range.end));
                }
            }
        }
        selection
    }

    fn candidate(
        &self,
        index: usize,
        range: Range<usize>,
        identity: Vec<Option<Identity>>,
    ) -> Candidate {
        let mut seen = IndexMap::<Identity, usize, Builder>::default();
        let mut resolution = Vec::with_capacity(range.len());
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

    // Applies a batch at once, each application naming its result with at most the allowance, and
    // gives each outcome with the steps its names took.
    fn attempt(
        &self,
        executor: Option<&Executor>,
        pending: &[(Identity, usize)],
        allowance: usize,
    ) -> Vec<(Outcome, usize)> {
        let _scope = profile::Scope::new(profile::Phase::Firing);
        map(
            executor,
            pending.iter().collect(),
            |(identity, position)| {
                let mut budget = allowance;
                let outcome = self.apply(
                    identity,
                    &self.trace[identity.source][*position],
                    &mut budget,
                );
                (outcome, allowance - budget)
            },
        )
    }

    // Names what an application made, spending the budget on its search; none when the budget runs
    // out, and the application then waits for the next round.
    fn analyze(&self, state: &State, budget: &mut usize) -> Option<Draft> {
        self.taxonomy.analyze(state, budget).ok()
    }

    // An event over parts applies to the parts it touches alone, once for every key, unless its
    // key made a configuration named whole; then, and for every other event, it applies to its
    // whole source.
    fn apply(&self, identity: &Identity, trace: &Trace, budget: &mut usize) -> Outcome {
        let (Some(layout), Owner::Frame(owner)) = (&self.layout[identity.source], &identity.owner)
        else {
            return self.configuration(identity, trace, None, budget);
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
            return self.configuration(identity, trace, None, budget);
        }
        let Some(product) = self.part(&local.key, budget) else {
            return Outcome::Deferred;
        };
        if matches!(product.draft, Draft::Whole(_)) {
            return self.configuration(identity, trace, Some(local.key), budget);
        }
        Outcome::Move(Box::new(Move {
            local,
            known: None,
            product: Some(product),
        }))
    }

    fn part(&self, key: &Key, budget: &mut usize) -> Option<Product> {
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
        Some(Product {
            draft: self.analyze(&result.state, budget)?,
            flow: result.flow,
            whole: None,
        })
    }

    // A rule a detached capture owns applies through the flow back to the configuration where
    // its match was found.
    fn configuration(
        &self,
        identity: &Identity,
        trace: &Trace,
        whole: Option<Key>,
        budget: &mut usize,
    ) -> Outcome {
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
        let Some(draft) = self.analyze(&result.state, budget) else {
            return Outcome::Deferred;
        };
        Outcome::Product(Box::new(Product {
            draft,
            flow: result.flow,
            whole,
        }))
    }

    // The environment a capture's rules see, spending the budget each time it serves. A
    // remembered failure is tried again only with a larger budget than it failed with, so whether
    // it fits never depends on which worker named it first.
    fn canonical(
        &self,
        origin: usize,
        frame: usize,
        budget: &mut usize,
    ) -> Result<Arc<Canonical>, Exhausted> {
        let allowance = *budget;
        let search = || {
            Name::search(allowance, |left| {
                self.state[origin].environment(frame, left)
            })
        };
        let found = match self.environment.get((origin, frame), |_| search()) {
            Name::Exhausted(tried) if tried < allowance => search(),
            found => found,
        };
        found.spend(budget)
    }

    // A trace's identity, or none when it no longer binds, spending the budget on naming the
    // environment of a capture that owns it.
    fn identify(
        &self,
        source: usize,
        trace: &Trace,
        budget: &mut usize,
    ) -> Result<Option<Identity>, Exhausted> {
        let Some(binding) = trace.binding(&self.state[source], &self.pool) else {
            return Ok(None);
        };
        let owner = match trace.owner() {
            Owner::Frame(frame) => Owner::Frame(frame),
            Owner::Capture(capture) => {
                let canonical = self.canonical(capture.origin, capture.frame, budget)?;
                Owner::Capture(Arc::new(capture.environment(&canonical)))
            }
        };
        Ok(Some(Identity {
            source,
            frame: trace.frame,
            owner,
            rule: trace.rule,
            binding,
        }))
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
                let mut budget = self.budget;
                let Ok(identity) = self.identify(state, &self.trace[state][position], &mut budget)
                else {
                    // The event this trace identifies cannot be found again, so the exploration
                    // never closes.
                    self.exhausted.store(true, Ordering::Relaxed);
                    return None;
                };
                self.identity[state].get(&identity?).copied()
            }
        }
    }
}

// Takes a batch's outcomes in turn, each paying a step for itself and the steps its names took,
// as long as they fit what the allowance leaves; the first that does not fit spends the rest, as a
// search that runs out does, and waits for the next round with every outcome after it. Each
// outcome's names were searched with what the allowance left for the batch's first, so an outcome
// that fits here fits as it would in turn.
fn spend(outcome: Vec<(Outcome, usize)>, used: &mut usize, allowance: usize) -> Vec<Outcome> {
    let mut spent = false;
    outcome
        .into_iter()
        .map(|(outcome, cost)| {
            if spent {
                return Outcome::Deferred;
            }
            *used += 1;
            if matches!(outcome, Outcome::Deferred) || *used + cost > allowance {
                *used = allowance;
                spent = true;
                return Outcome::Deferred;
            }
            *used += cost;
            outcome
        })
        .collect()
}
