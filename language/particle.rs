use crate::state::Token;
use crate::term::Term;
use smallvec::SmallVec;
use std::task::Poll;

struct Group {
    candidate: Vec<usize>,
    position: Vec<usize>,
    selected: Vec<usize>,
}

impl Group {
    fn advance(&mut self) -> bool {
        let width = self.selected.len();
        let Some(position) = (0..width)
            .rev()
            .find(|&position| self.selected[position] < self.candidate.len() - width + position)
        else {
            for (index, selected) in self.selected.iter_mut().enumerate() {
                *selected = index;
            }
            return false;
        };
        self.selected[position] += 1;
        for index in position + 1..width {
            self.selected[index] = self.selected[index - 1] + 1;
        }
        true
    }
}

pub(crate) struct Match {
    group: Vec<Group>,
    width: usize,
    fresh: bool,
    complete: bool,
}

impl Match {
    pub(crate) fn new(pattern: &[Term], particle: &[Token]) -> Self {
        let mut group: Vec<Group> = Vec::new();
        let mut known: SmallVec<[(&Term, usize); 4]> = SmallVec::new();
        for (position, term) in pattern.iter().enumerate() {
            if let Some(&(_, index)) = known.iter().find(|&&(value, _)| value == term) {
                let group = &mut group[index];
                group.position.push(position);
                group.selected.push(group.selected.len());
                continue;
            }
            let mut candidate = particle
                .iter()
                .filter(|token| term.matches(token))
                .map(|token| token.id)
                .collect::<Vec<_>>();
            candidate.sort_unstable();
            candidate.dedup();
            if let Some(index) = group.iter().position(|group| group.candidate == candidate) {
                known.push((term, index));
                let group = &mut group[index];
                group.position.push(position);
                group.selected.push(group.selected.len());
            } else {
                known.push((term, group.len()));
                group.push(Group {
                    candidate,
                    position: vec![position],
                    selected: vec![0],
                });
            }
        }
        let complete = group
            .iter()
            .any(|group| group.candidate.len() < group.selected.len());
        Self {
            group,
            width: pattern.len(),
            fresh: true,
            complete,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.fresh = true;
        self.complete = self
            .group
            .iter()
            .any(|group| group.candidate.len() < group.selected.len());
        for group in &mut self.group {
            for (index, selected) in group.selected.iter_mut().enumerate() {
                *selected = index;
            }
        }
    }

    pub(crate) fn step(&mut self) -> Poll<Option<Vec<usize>>> {
        if self.complete {
            return Poll::Ready(None);
        }
        if !self.fresh && !self.group.iter_mut().rev().any(Group::advance) {
            self.complete = true;
            return Poll::Ready(None);
        }
        self.fresh = false;
        let mut result = vec![0; self.width];
        for group in &self.group {
            for (&position, &selected) in group.position.iter().zip(&group.selected) {
                result[position] = group.candidate[selected];
            }
        }
        Poll::Ready(Some(result))
    }

    pub(crate) fn retained(&self) -> usize {
        self.group
            .iter()
            .map(|group| group.candidate.len() + group.position.len() + group.selected.len())
            .sum()
    }
}

#[cfg(test)]
#[path = "test/particle.rs"]
mod test;
