use crate::state::Token;
use crate::term::Term;
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
        for (position, term) in pattern.iter().enumerate() {
            let mut candidate = particle
                .iter()
                .filter(|token| term.matches(token))
                .map(|token| token.id)
                .collect::<Vec<_>>();
            candidate.sort_unstable();
            candidate.dedup();
            if let Some(group) = group.iter_mut().find(|group| group.candidate == candidate) {
                group.position.push(position);
                group.selected.push(group.selected.len());
            } else {
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
