mod cursor;
mod dependency;
mod key;
mod layer;
mod node;
mod partition;
mod playback;
mod prefix;
mod product;
mod recording;
mod retention;
mod space;
mod store;
mod strategy;
mod stream;
mod trace;
mod tree;

#[cfg(test)]
#[path = "test/batch.rs"]
mod batch;

pub(crate) use store::Store;

use crate::index::Index;
use crate::slot::Slot;
#[cfg(test)]
use crate::term::Term;
use cursor::Cursor;
use partition::Partition;
use product::Product;
use smallvec::SmallVec;
use space::Space;
use std::sync::Arc;
use std::task::Poll;
use strategy::Strategy;
use stream::Stream;
use tree::Tree;

#[repr(u8)]
enum Traversal {
    Direct(Cursor),
    Factored(Box<Product<Stream>>),
    Partitioned(Box<Product<Partition>>),
    Layered(Box<Product<Tree>>),
}

impl Traversal {
    fn retained(&self) -> usize {
        match self {
            Self::Direct(cursor) => cursor.retained(),
            Self::Factored(product) => product.retained(),
            Self::Partitioned(product) => product.retained(),
            Self::Layered(product) => product.retained(),
        }
    }
}

pub(crate) struct Request<'a> {
    pub input: &'a crate::plan::Input,
    pub index: &'a Index,
    pub frame: usize,
    pub owner: usize,
    pub store: &'a Arc<Store>,
}

pub(crate) struct Join {
    space: Space,
    order: SmallVec<[usize; 2]>,
    traversal: Traversal,
    viable: bool,
    complete: bool,
    stable: bool,
}

impl Join {
    pub fn waiting(&self) -> usize {
        if self.complete {
            return 0;
        }
        match &self.traversal {
            Traversal::Direct(_) => 0,
            Traversal::Factored(product) => product.waiting(),
            Traversal::Partitioned(product) => product.waiting(),
            Traversal::Layered(product) => product.waiting(),
        }
    }

    pub fn skip(&mut self, maximum: usize) -> usize {
        if self.complete {
            return 0;
        }
        match &mut self.traversal {
            Traversal::Direct(_) => 0,
            Traversal::Factored(product) => product.skip(maximum),
            Traversal::Partitioned(product) => product.skip(maximum),
            Traversal::Layered(product) => product.skip(maximum),
        }
    }

    #[cfg(test)]
    pub fn new(pattern: Arc<Vec<Vec<Term>>>, index: &Index, frame: usize) -> Self {
        Self::construct(Space::new(pattern, index, frame, None, None), index)
    }

    fn construct(space: Space, index: &Index) -> Self {
        let mut order: SmallVec<[usize; 2]> = (0..space.pattern.len()).collect();
        order.sort_by_key(|&position| space.domain[position].len());
        let traversal = Traversal::Direct(Cursor::new(order.len()));
        let mut join = Self {
            space,
            order,
            traversal,
            viable: false,
            complete: false,
            stable: false,
        };
        join.viable = join.feasible();
        join.reset(index);
        join
    }

    fn product(space: &Space, order: &[usize], strategy: Strategy) -> Option<Traversal> {
        if let Some(store) = &space.store
            && order.len() >= 3
            && order[..order.len() - 1]
                .iter()
                .any(|&position| space.pattern[position].len() >= 8)
        {
            let width = order.len() - 1;
            return Some(match strategy {
                Strategy::Shared => {
                    let node = store.subscribe(key::Key::new(space, &order[..width]));
                    Traversal::Factored(Box::new(Product::new(Stream::new(
                        width,
                        store.budget().clone(),
                        node,
                    ))))
                }
                Strategy::Layered { previous, depth } => Traversal::Layered(Box::new(
                    Product::new(Tree::new(width, [previous, depth], store.budget().clone())),
                )),
                Strategy::Partitioned(depth) => Traversal::Partitioned(Box::new(Product::new(
                    Partition::new(width, depth, store.budget().clone()),
                ))),
            });
        }
        None
    }

    pub fn planned(request: Request<'_>) -> Self {
        Self::construct(
            Space::new(
                request.input.pattern(request.owner),
                request.index,
                request.frame,
                Some(request.input.context(request.owner)),
                Some(request.store),
            ),
            request.index,
        )
    }

    #[cfg(test)]
    pub fn advance(&mut self, index: &Index) {
        self.update(index);
        self.reset(index);
    }

    pub fn update(&mut self, index: &Index) -> bool {
        let changed = self.space.update(index);
        if changed.is_empty() {
            return false;
        }
        let previous = self.order.clone();
        self.order
            .sort_by_key(|&position| self.space.domain[position].len());
        let strategy = (self.order == previous)
            .then(|| {
                strategy::select(strategy::Request {
                    space: &self.space,
                    order: &self.order,
                    changed: &changed,
                    index,
                    history: match &self.traversal {
                        Traversal::Partitioned(product) => Some(strategy::History::Single {
                            depth: product.prefix.depth(),
                            granularity: product.prefix.granularity(),
                        }),
                        Traversal::Layered(product) => {
                            Some(strategy::History::Nested(product.prefix.depth()))
                        }
                        _ => None,
                    },
                })
            })
            .flatten();
        if let Some(strategy) = strategy {
            let replace = match (&self.traversal, strategy) {
                (Traversal::Direct(_), _) => self.stable,
                (Traversal::Factored(_), Strategy::Partitioned(_)) => true,
                (Traversal::Partitioned(_), Strategy::Layered { .. }) => true,
                (Traversal::Partitioned(product), Strategy::Partitioned(depth)) => {
                    product.prefix.depth() != depth
                }
                _ => false,
            };
            if replace && let Some(product) = Self::product(&self.space, &self.order, strategy) {
                self.traversal = product;
            }
            if let Traversal::Layered(product) = &mut self.traversal {
                product.prefix.update(tree::Update {
                    index,
                    order: &self.order,
                    changed: &changed,
                    depth: match strategy {
                        Strategy::Layered { depth, .. } => Some(depth),
                        _ => None,
                    },
                });
            }
            if matches!(strategy, Strategy::Partitioned(_))
                && let Traversal::Partitioned(product) = &mut self.traversal
            {
                product.prefix.update(index);
            }
        } else if !matches!(self.traversal, Traversal::Direct(_)) || self.order != previous {
            self.traversal = Traversal::Direct(Cursor::new(self.order.len()));
        }
        self.stable = strategy.is_some();
        self.viable = self.feasible();
        self.reset(index);
        true
    }

    pub fn reset(&mut self, index: &Index) {
        match &mut self.traversal {
            Traversal::Direct(cursor) => cursor.reset(),
            Traversal::Factored(product) => product.reset(index),
            Traversal::Partitioned(product) => product.reset(index),
            Traversal::Layered(product) => product.reset(index),
        }
        self.complete = self.space.domain.iter().any(Vec::is_empty);
    }

    pub fn viable(&self) -> bool {
        self.viable
    }
    pub fn frame(&self) -> usize {
        self.space.frame
    }

    fn feasible(&self) -> bool {
        if self.space.domain.iter().any(Vec::is_empty) {
            return false;
        }
        if self.space.domain.len() <= 1 {
            return true;
        }
        let mut selected = Vec::with_capacity(self.space.domain.len());
        for &position in &self.order {
            if let Some(member) = self.space.domain[position]
                .iter()
                .find(|member| !selected.contains(&member.site))
            {
                selected.push(member.site);
            } else {
                break;
            }
        }
        if selected.len() == self.space.domain.len() {
            return true;
        }
        let domain = self
            .space
            .domain
            .iter()
            .map(|domain| domain.iter().map(|member| member.site).collect())
            .collect::<Vec<Vec<_>>>();
        domain.iter().all(|domain| !domain.is_empty()) && crate::assignment::feasible(&domain)
    }

    #[inline]
    pub fn step(&mut self, index: &Index) -> Poll<Option<Vec<Slot>>> {
        if self.complete {
            return Poll::Ready(None);
        }
        let result = match &mut self.traversal {
            Traversal::Direct(cursor) => cursor.step(&mut self.space, &self.order, index),
            Traversal::Factored(product) => product.step(&mut self.space, &self.order, index),
            Traversal::Partitioned(product) => product.step(&mut self.space, &self.order, index),
            Traversal::Layered(product) => product.step(&mut self.space, &self.order, index),
        };
        result.map(|selection| {
            selection.map(|mut selection| {
                for slot in &mut selection {
                    slot.world = index.world(slot.world);
                }
                selection.sort_by_key(|slot| slot.position);
                selection
            })
        })
    }

    pub fn evict(&mut self) {
        match &mut self.traversal {
            Traversal::Direct(_) => {}
            Traversal::Factored(product) => product.evict(),
            Traversal::Partitioned(product) => product.evict(),
            Traversal::Layered(product) => product.evict(),
        }
        self.space.evict();
    }

    #[cfg(test)]
    fn size(&self) -> usize {
        self.space.size()
            + self.order.len()
            + match &self.traversal {
                Traversal::Direct(cursor) => cursor.size(),
                Traversal::Factored(product) => product.size(),
                Traversal::Partitioned(product) => product.size(),
                Traversal::Layered(product) => product.size(),
            }
    }

    pub fn retained(&self) -> usize {
        self.space.retained() + self.order.len() + self.traversal.retained()
    }
}

#[cfg(test)]
#[path = "test/factorization.rs"]
mod test;

#[cfg(test)]
#[path = "test/sharing.rs"]
mod sharing;

#[cfg(test)]
#[path = "test/domain.rs"]
mod domain;

#[cfg(test)]
#[path = "test/fragment.rs"]
mod fragment;

#[cfg(test)]
#[path = "test/tree.rs"]
mod hierarchy;
