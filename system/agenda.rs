use std::collections::VecDeque;

pub(crate) struct Queue<Value> {
    ready: VecDeque<Value>,
    deferred: VecDeque<Value>,
    credit: usize,
}

impl<Value> Queue<Value> {
    pub(crate) fn new() -> Self {
        Self {
            ready: VecDeque::new(),
            deferred: VecDeque::new(),
            credit: 4096,
        }
    }

    pub(crate) fn push_back(&mut self, value: Value) {
        self.ready.push_back(value);
    }

    pub(crate) fn defer(&mut self, value: Value) {
        self.deferred.push_back(value);
    }

    pub(crate) fn extend(&mut self, value: impl IntoIterator<Item = Value>) {
        self.ready.extend(value);
    }

    fn immediate(&self) -> bool {
        !self.ready.is_empty() && (self.credit != 0 || self.deferred.is_empty())
    }

    pub(crate) fn front(&self) -> Option<&Value> {
        if self.immediate() {
            self.ready.front()
        } else {
            self.deferred.front()
        }
    }

    pub(crate) fn pop_front(&mut self) -> Option<Value> {
        if self.immediate() {
            self.credit = self.credit.saturating_sub(1);
            self.ready.pop_front()
        } else {
            self.credit = 4096;
            self.deferred.pop_front()
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.ready.len() + self.deferred.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.ready.is_empty() && self.deferred.is_empty()
    }
}

#[cfg(test)]
#[path = "test/agenda.rs"]
mod test;
