use super::firing::{Move, Outcome, Product};
use super::layout::Layout;
use super::passage::{Composed, Origin, Passage};
use super::space::{self, Found};
use super::taxonomy::{Makeup, Taxonomy};
use super::transition::{Effect, Transition};
use super::{Event, Identity, Laser, Round, map};
use crate::executor::Executor;
use crate::profile;
use crate::state::State;
use smallvec::SmallVec;
use std::sync::Arc;

enum Route {
    Flat(Passage),
    Composed {
        source: usize,
        origin: Vec<Origin>,
        involved: SmallVec<[usize; 4]>,
        transition: Arc<Transition>,
    },
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

fn learn(taxonomy: &Taxonomy, mut value: Box<Move>, root: u32, kind: &[u32]) -> Box<Move> {
    let Product {
        draft,
        flow,
        original,
        ..
    } = value
        .product
        .take()
        .expect("a move without its transition applied itself");
    let (makeup, renaming) = taxonomy.assemble(draft, root, kind, original);
    let extent = taxonomy.extent(&makeup);
    let key = &value.local.key;
    value.known = Some(Arc::new(Transition::Local(Box::new(Effect {
        root: makeup.root,
        source: Layout::of(taxonomy, key.root, key.kind.iter().copied()),
        result: Layout::new(taxonomy, &makeup),
        passage: Passage::new(renaming.flow(flow, extent.0, extent.1)),
        produced: makeup.kind,
    }))));
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
    pub(super) fn create(
        &mut self,
        executor: Option<&Executor>,
        pending: Vec<(Identity, usize, usize)>,
        outcome: Vec<Outcome>,
        next: &mut Round,
    ) -> Vec<Option<usize>> {
        let _scope = profile::Scope::new(profile::Phase::Creation);
        let mut number = Vec::with_capacity(outcome.len());
        for outcome in &outcome {
            number.push(match outcome {
                Outcome::Product(product) => Some(self.taxonomy.intern(&product.draft)),
                Outcome::Move(value) => value
                    .product
                    .as_ref()
                    .map(|product| self.taxonomy.intern(&product.draft)),
                Outcome::Blocked => None,
            });
        }
        let taxonomy = &self.taxonomy;
        let mut learned = map(
            executor,
            outcome.into_iter().zip(number).collect(),
            |(outcome, number)| match outcome {
                Outcome::Move(value) if value.product.is_some() => {
                    let (root, kind) = number.expect("a product is numbered");
                    (Outcome::Move(learn(taxonomy, value, root, &kind)), None)
                }
                other => (other, number),
            },
        );
        for (outcome, _) in &mut learned {
            match outcome {
                Outcome::Move(value) => {
                    let found = value.known.take().expect("a move knows its transition");
                    let key = value.local.key.clone();
                    value.known = Some(self.memo.entry(key).or_insert(found).clone());
                }
                Outcome::Product(product) => {
                    if let Some(key) = product.whole.take() {
                        self.memo
                            .entry(key)
                            .or_insert_with(|| Arc::new(Transition::Whole));
                    }
                }
                Outcome::Blocked => {}
            }
        }
        let taxonomy = &self.taxonomy;
        let makeup = &self.makeup;
        let limit = self.limit;
        let named = map(
            executor,
            learned
                .into_iter()
                .zip(pending.iter().map(|&(_, source, _)| source))
                .collect(),
            |((outcome, number), source)| match outcome {
                Outcome::Product(product) => {
                    let (root, kind) = number.expect("a product is numbered");
                    let Product {
                        draft,
                        flow,
                        original,
                        ..
                    } = *product;
                    let (makeup, renaming) = taxonomy.assemble(draft, root, &kind, original);
                    let extent = taxonomy.extent(&makeup);
                    let passage = Passage::new(renaming.flow(flow, extent.0, extent.1));
                    Some(Named {
                        hash: space::hash(&makeup),
                        makeup,
                        route: Route::Flat(passage),
                    })
                }
                Outcome::Move(value) => {
                    let Move { local, known, .. } = *value;
                    let transition = known.expect("a move knows its transition");
                    let Transition::Local(effect) = &*transition else {
                        unreachable!("a move holds a local transition")
                    };
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
                            transition,
                        },
                    })
                }
                Outcome::Blocked => None,
            },
        );
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
        let settled = map(executor, paired, |(named, found)| match (named, found) {
            (Some(named), Some(Found::Existing(index))) => {
                Settled::Existing(index, Box::new(named.route))
            }
            (Some(named), Some(Found::Repeat(earlier))) => {
                Settled::Repeat(earlier, Box::new(named.route))
            }
            (Some(named), Some(Found::New)) => {
                let state = Arc::new(taxonomy.materialize(&named.makeup));
                let layout = taxonomy
                    .hub(&named.makeup)
                    .then(|| Arc::new(Layout::new(taxonomy, &named.makeup)));
                Settled::New(
                    named.hash,
                    Arc::new(named.makeup),
                    state,
                    layout,
                    Box::new(named.route),
                )
            }
            _ => Settled::Blocked,
        });
        let mut target = Vec::<Option<usize>>::new();
        let mut admitted = Vec::new();
        let mut created = Vec::with_capacity(pending.len());
        let mut fired = Vec::new();
        for ((identity, source, position), settled) in pending.into_iter().zip(settled) {
            let (resolved, route) = match settled {
                Settled::Blocked => {
                    self.blocked.insert(identity, (source, position));
                    created.push(None);
                    continue;
                }
                Settled::Existing(index, route) => (Some(index), route),
                Settled::Repeat(earlier, route) => (target[earlier], route),
                Settled::New(..) if self.state.len() >= self.limit.configuration => {
                    target.push(None);
                    self.blocked.insert(identity, (source, position));
                    created.push(None);
                    continue;
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
                self.blocked.insert(identity, (source, position));
                created.push(None);
                continue;
            };
            let event = self.event.len();
            self.crossed.push(Vec::new());
            let passage = match *route {
                Route::Flat(passage) => passage,
                Route::Composed {
                    source,
                    origin,
                    involved,
                    transition,
                } => Passage::Composed(Composed {
                    source: self.layout[source]
                        .clone()
                        .expect("a move starts from a hub"),
                    target: self.layout[resolved].clone().expect("a move ends at a hub"),
                    origin: origin.into_boxed_slice(),
                    involved,
                    transition,
                }),
            };
            self.passage.push(passage);
            fired.push((source, event, identity));
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
        fired.sort_by_key(|&(source, event, _)| (source, event));
        let mut group = Vec::<(usize, Vec<(Identity, usize)>)>::new();
        for (source, event, identity) in fired {
            match group.last_mut() {
                Some((index, list)) if *index == source => list.push((identity, event)),
                _ => group.push((source, vec![(identity, event)])),
            }
        }
        for (index, list) in &group {
            let base = self.identity[*index].len();
            for (offset, &(_, event)) in list.iter().enumerate() {
                self.event[event].slot = base + offset;
            }
        }
        let taken = group
            .into_iter()
            .map(|(index, list)| (index, std::mem::take(&mut self.identity[index]), list))
            .collect::<Vec<_>>();
        let inserted = map(executor, taken, |(index, mut table, list)| {
            table.extend(list);
            (index, table)
        });
        for (index, table) in inserted {
            self.identity[index] = table;
        }
        created
    }
}
