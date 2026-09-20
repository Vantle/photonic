mod cursor;
mod product;
mod space;
mod stream;

use crate::index::Index;
use crate::slot::Slot;
#[cfg(test)]
use crate::term::Term;
use cursor::Cursor;
use product::Product;
use smallvec::SmallVec;
use space::Space;
use std::sync::Arc;
use std::task::Poll;

enum Traversal {
    Direct(Cursor),
    Factored(Box<Product>),
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
    #[cfg(test)]
    pub fn new(pattern: Arc<Vec<Vec<Term>>>, index: &Index, frame: usize) -> Self {
        Self::construct(Space::new(pattern, index, frame, None, None))
    }

    fn construct(space: Space) -> Self {
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
        join.reset();
        join
    }

    fn product(space: &Space, order: &[usize]) -> Option<Box<Product>> {
        if let Some(budget) = &space.budget
            && order.len() >= 3
            && order[..order.len() - 1]
                .iter()
                .any(|&position| space.pattern[position].len() >= 8)
        {
            return Some(Box::new(Product::new(order.len(), budget.clone())));
        }
        None
    }

    pub fn planned(
        input: &crate::plan::Input,
        index: &Index,
        frame: usize,
        owner: usize,
        budget: &Arc<crate::factor::Budget>,
    ) -> Self {
        Self::construct(Space::new(
            input.pattern(owner),
            index,
            frame,
            Some(input.context(owner)),
            input.factor().then(|| budget.clone()),
        ))
    }

    #[cfg(test)]
    pub fn advance(&mut self, index: &Index) {
        self.update(index);
        self.reset();
    }

    pub fn update(&mut self, index: &Index) -> bool {
        let changed = self.space.update(index);
        if changed.is_empty() {
            return false;
        }
        let previous = self.order.clone();
        self.order
            .sort_by_key(|&position| self.space.domain[position].len());
        let stable = self.order == previous
            && !changed.iter().any(|position| {
                self.order[..self.order.len().saturating_sub(1)].contains(position)
            });
        if !stable {
            if matches!(self.traversal, Traversal::Factored(_)) || self.order != previous {
                self.traversal = Traversal::Direct(Cursor::new(self.order.len()));
            }
        } else if self.stable
            && matches!(self.traversal, Traversal::Direct(_))
            && let Some(product) = Self::product(&self.space, &self.order)
        {
            self.traversal = Traversal::Factored(product);
        }
        self.stable = stable;
        self.viable = self.feasible();
        self.reset();
        true
    }

    pub fn reset(&mut self) {
        match &mut self.traversal {
            Traversal::Direct(cursor) => cursor.reset(),
            Traversal::Factored(product) => product.reset(),
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

    pub fn step(&mut self, index: &Index) -> Poll<Option<Vec<Slot>>> {
        if self.complete {
            return Poll::Ready(None);
        }
        let result = match &mut self.traversal {
            Traversal::Direct(cursor) => cursor.step(&mut self.space, &self.order, index),
            Traversal::Factored(product) => product.step(&mut self.space, &self.order, index),
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
        if let Traversal::Factored(product) = &mut self.traversal {
            product.evict();
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
            }
    }

    pub fn retained(&self) -> usize {
        self.space.retained()
            + self.order.len()
            + match &self.traversal {
                Traversal::Direct(cursor) => cursor.retained(),
                Traversal::Factored(product) => product.retained(),
            }
    }
}

#[cfg(test)]
#[path = "test/factorization.rs"]
mod test;
