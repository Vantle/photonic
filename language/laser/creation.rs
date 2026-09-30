use super::firing::{Move, Outcome, Product};
use super::layout::Layout;
use super::makeup::Makeup;
use super::passage::{Composed, Flat, Origin, Passage};
use super::space::{self, Found};
use super::taxonomy::Taxonomy;
use super::transition::{Effect, Key};
use super::{Event, Identity, Laser, Round, build, map, update};
use crate::executor::Executor;
use crate::profile;
use crate::state::State;
use smallvec::SmallVec;
use std::collections::hash_map::Entry;
use std::sync::Arc;

enum Route {
    Flat(Box<Flat>),
    Composed {
        source: usize,
        origin: Vec<Origin>,
        involved: SmallVec<[usize; 4]>,
        effect: Arc<Effect>,
    },
}

// What an application made, with the numbers its product's root and parts took.
struct Learned {
    outcome: Outcome,
    number: Option<(u32, Vec<u32>)>,
}

struct Named {
    hash: u64,
    makeup: Makeup,
    route: Route,
}

enum Settled {
    Blocked,
    Existing(usize, Box<Route>),
    Repeat(usize, Box<Route>),
    New(
        u64,
        Arc<Makeup>,
        Arc<State>,
        Option<Arc<Layout>>,
        Box<Route>,
    ),
}

// A numbered product's makeup, and the passage back to what it was applied to.
fn realize(taxonomy: &Taxonomy, product: Product, root: u32, kind: &[u32]) -> (Makeup, Flat) {
    let (makeup, renaming) = taxonomy.assemble(product.draft, root, kind);
    let extent = taxonomy.extent(&makeup);
    (
        makeup,
        Flat::new(renaming.flow(product.flow, extent.world, extent.frame)),
    )
}

// A move that applied itself to its parts learns its effect from what it made of them.
fn effect(taxonomy: &Taxonomy, mut value: Box<Move>, root: u32, kind: &[u32]) -> Box<Move> {
    let product = value
        .product
        .take()
        .expect("a move without its effect applied itself");
    let (makeup, passage) = realize(taxonomy, product, root, kind);
    let key = &value.local.key;
    value.known = Some(Arc::new(Effect {
        root: makeup.root,
        source: Layout::of(taxonomy, key.root, &key.kind),
        result: Layout::new(taxonomy, &makeup),
        passage,
        produced: makeup.kind,
    }));
    value
}

fn merge(
    source: &Makeup,
    involved: &[usize],
    produced: &[u32],
    root: u32,
) -> (Makeup, Vec<Origin>) {
    let mut kind = Vec::with_capacity(source.kind.len() + produced.len());
    let mut origin = Vec::with_capacity(source.kind.len() + produced.len());
    let mut kept = (0..source.kind.len())
        .filter(|part| involved.binary_search(part).is_err())
        .peekable();
    let mut made = produced.iter().copied().enumerate().peekable();
    loop {
        let keep = match (kept.peek(), made.peek()) {
            (Some(&part), Some(&(_, value))) => source.kind[part] <= value,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };
        if keep {
            let part = kept.next().expect("a kept part was seen");
            kind.push(source.kind[part]);
            origin.push(Origin::Same(
                u32::try_from(part).expect("fewer than 2^32 parts"),
            ));
        } else {
            let (index, value) = made.next().expect("a made part was seen");
            kind.push(value);
            origin.push(Origin::Produced(
                u32::try_from(index).expect("fewer than 2^32 parts"),
            ));
        }
    }
    (Makeup { root, kind }, origin)
}

impl Laser {
    // Names what a batch of applications made, finds or makes the configurations they reach and
    // records their events, in the batch's order.
    pub(super) fn create(
        &mut self,
        executor: Option<&Executor>,
        pending: Vec<(Identity, usize)>,
        outcome: Vec<Outcome>,
        next: &mut Round,
    ) -> Vec<Option<usize>> {
        let _scope = profile::Scope::new(profile::Phase::Creation);
        let deferred = outcome
            .iter()
            .map(|outcome| matches!(outcome, Outcome::Deferred))
            .collect();
        let (learned, fresh) = self.learn(executor, outcome);
        let named = self.label(executor, learned, &pending);
        let settled = self.locate(executor, named);
        let (created, fired) = self.commit(executor, pending, settled, deferred, next);
        self.register(executor, fired);
        self.prune(fresh);
        created
    }

    // Numbers every product in the batch's order, learns the effect of each move that applied
    // itself to its parts, and remembers every effect and every key named whole; gives the keys
    // whose effects this batch learned.
    fn learn(
        &mut self,
        executor: Option<&Executor>,
        outcome: Vec<Outcome>,
    ) -> (Vec<Learned>, Vec<Key>) {
        let number = outcome
            .iter()
            .map(|outcome| match outcome {
                Outcome::Product(product) => Some(self.taxonomy.intern(&product.draft)),
                Outcome::Move(value) => value
                    .product
                    .as_ref()
                    .map(|product| self.taxonomy.intern(&product.draft)),
                Outcome::Blocked | Outcome::Deferred => None,
            })
            .collect::<Vec<_>>();
        let taxonomy = &self.taxonomy;
        let mut learned = map(
            executor,
            outcome.into_iter().zip(number).collect(),
            |(outcome, number)| match outcome {
                Outcome::Move(value) if value.product.is_some() => {
                    let (root, kind) = number.expect("a product is numbered");
                    Learned {
                        outcome: Outcome::Move(effect(taxonomy, value, root, &kind)),
                        number: None,
                    }
                }
                outcome => Learned { outcome, number },
            },
        );
        let mut fresh = Vec::new();
        for Learned { outcome, .. } in &mut learned {
            match outcome {
                Outcome::Move(value) => {
                    let found = value.known.take().expect("a move knows its effect");
                    let known = match self.memo.entry(value.local.key.clone()) {
                        Entry::Occupied(entry) => entry.get().clone(),
                        Entry::Vacant(entry) => {
                            fresh.push(entry.key().clone());
                            entry.insert(found).clone()
                        }
                    };
                    value.known = Some(known);
                }
                Outcome::Product(product) => {
                    if let Some(key) = product.whole.take() {
                        self.whole.insert(key);
                    }
                }
                Outcome::Blocked | Outcome::Deferred => {}
            }
        }
        (learned, fresh)
    }

    // An effect this batch learned that no event it created holds was learned for events the
    // limits refused, and remembering it would keep an effect for every refused application.
    fn prune(&mut self, fresh: Vec<Key>) {
        for key in fresh {
            if self
                .memo
                .get(&key)
                .is_some_and(|effect| Arc::strong_count(effect) == 1)
            {
                self.memo.remove(&key);
            }
        }
    }

    // Each result's makeup, hash and passage back to its source; a move's result the limits
    // refuse has none.
    fn label(
        &self,
        executor: Option<&Executor>,
        learned: Vec<Learned>,
        pending: &[(Identity, usize)],
    ) -> Vec<Option<Named>> {
        let taxonomy = &self.taxonomy;
        let makeup = &self.makeup;
        let limit = self.limit;
        let source = pending.iter().map(|(identity, _)| identity.source);
        map(
            executor,
            learned.into_iter().zip(source).collect(),
            |(Learned { outcome, number }, source)| match outcome {
                Outcome::Product(product) => {
                    let (root, kind) = number.expect("a product is numbered");
                    let (makeup, passage) = realize(taxonomy, *product, root, &kind);
                    Some(Named {
                        hash: space::hash(&makeup),
                        makeup,
                        route: Route::Flat(Box::new(passage)),
                    })
                }
                Outcome::Move(value) => {
                    let Move { local, known, .. } = *value;
                    let effect = known.expect("a move knows its effect");
                    let (makeup, origin) = merge(
                        &makeup[source],
                        &local.involved,
                        &effect.produced,
                        effect.root,
                    );
                    let (coherence, occurrence, scope) = taxonomy.measure(&makeup);
                    if !limit.admits(coherence, occurrence, scope) {
                        return None;
                    }
                    Some(Named {
                        hash: space::hash(&makeup),
                        makeup,
                        route: Route::Composed {
                            source,
                            origin,
                            involved: local.involved,
                            effect,
                        },
                    })
                }
                Outcome::Blocked | Outcome::Deferred => None,
            },
        )
    }

    // Where each result lies: a configuration already known, one an earlier result of the batch
    // makes, or a new one, materialized here in parallel.
    fn locate(&self, executor: Option<&Executor>, named: Vec<Option<Named>>) -> Vec<Settled> {
        let item = named
            .iter()
            .flatten()
            .map(|named| (named.hash, &named.makeup))
            .collect::<Vec<_>>();
        let mut found = self.space.resolve(executor, &item).into_iter();
        let paired = named
            .into_iter()
            .map(|named| {
                let found = named
                    .is_some()
                    .then(|| found.next().expect("one answer for each applied event"));
                (named, found)
            })
            .collect::<Vec<_>>();
        let taxonomy = &self.taxonomy;
        map(executor, paired, |(named, found)| match (named, found) {
            (Some(named), Some(Found::Existing(index))) => {
                Settled::Existing(index, Box::new(named.route))
            }
            (Some(named), Some(Found::Repeat(earlier))) => {
                Settled::Repeat(earlier, Box::new(named.route))
            }
            (Some(named), Some(Found::New)) => {
                let (state, layout) = build(taxonomy, &named.makeup);
                Settled::New(
                    named.hash,
                    Arc::new(named.makeup),
                    state,
                    layout,
                    Box::new(named.route),
                )
            }
            _ => Settled::Blocked,
        })
    }

    // Adds the new configurations and the events in the batch's order, blocking an identity whose
    // result the limits refuse and deferring one whose result the round could not name; gives each
    // identity's event and the events fired.
    fn commit(
        &mut self,
        executor: Option<&Executor>,
        pending: Vec<(Identity, usize)>,
        settled: Vec<Settled>,
        deferred: Vec<bool>,
        next: &mut Round,
    ) -> (Vec<Option<usize>>, Vec<(usize, Identity)>) {
        let mut target = Vec::<Option<usize>>::new();
        let mut admitted = Vec::new();
        let mut created = Vec::with_capacity(pending.len());
        let mut fired = Vec::new();
        for (((identity, position), settled), deferred) in
            pending.into_iter().zip(settled).zip(deferred)
        {
            let (resolved, route) = match settled {
                Settled::Blocked if deferred => {
                    self.blocked.insert(identity.clone(), position);
                    next.retry.push((identity, position));
                    created.push(None);
                    continue;
                }
                Settled::Blocked => {
                    self.blocked.insert(identity, position);
                    created.push(None);
                    continue;
                }
                Settled::Existing(index, route) => (Some(index), route),
                Settled::Repeat(earlier, route) => (target[earlier], route),
                Settled::New(.., route) if self.state.len() >= self.limit.configuration => {
                    (None, route)
                }
                Settled::New(hash, makeup, state, layout, route) => {
                    let index = self.push(state, makeup.clone(), layout);
                    admitted.push((hash, makeup, index));
                    next.fresh.push(index);
                    (Some(index), route)
                }
            };
            target.push(resolved);
            let Some(resolved) = resolved else {
                self.blocked.insert(identity, position);
                created.push(None);
                continue;
            };
            let source = identity.source;
            let event = self.event.len();
            self.crossed.push(Vec::new());
            let passage = match *route {
                Route::Flat(flat) => Passage::Flat(flat),
                Route::Composed {
                    source,
                    origin,
                    involved,
                    effect,
                } => Passage::Composed(Composed {
                    source: self.layout[source]
                        .clone()
                        .expect("a move starts from a hub"),
                    target: self.layout[resolved].clone().expect("a move ends at a hub"),
                    origin: origin.into_boxed_slice(),
                    involved,
                    effect,
                }),
            };
            self.passage.push(passage);
            fired.push((event, identity));
            self.event.push(Event {
                source,
                slot: 0,
                target: resolved,
                direct: position < self.origin[source],
                matched: position < self.origin[source],
            });
            self.incoming[resolved].push(event);
            next.changed.push(resolved);
            created.push(Some(event));
        }
        self.space.admit(executor, admitted);
        (created, fired)
    }

    // Numbers each fired event's identity at its source in the order the events fired, and adds
    // the identities to their sources' tables in parallel.
    fn register(&mut self, executor: Option<&Executor>, mut fired: Vec<(usize, Identity)>) {
        fired.sort_by_key(|(event, identity)| (identity.source, *event));
        let mut group = Vec::<(usize, Vec<(Identity, usize)>)>::new();
        for (event, identity) in fired {
            match group.last_mut() {
                Some((source, list)) if *source == identity.source => list.push((identity, event)),
                _ => group.push((identity.source, vec![(identity, event)])),
            }
        }
        for (source, list) in &group {
            let base = self.identity[*source].len();
            for (offset, &(_, event)) in list.iter().enumerate() {
                self.event[event].slot = base + offset;
            }
        }
        update(executor, &mut self.identity, group, |table, list| {
            table.extend(list);
        });
    }
}
