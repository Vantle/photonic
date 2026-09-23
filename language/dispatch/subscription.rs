use super::consumer::Request;
use super::entry::Entry;
use super::{Key, Network};
use crate::index::Index;
use crate::mask::Set;
use crate::profile;
use crate::replay::Search;
use smallvec::SmallVec;

impl Network {
    pub(super) fn frame(&mut self, index: &Index, frame: usize, selected: Option<&Set>) {
        let _scope = profile::Scope::new(profile::Phase::Subscription);
        let mut request = std::mem::take(&mut self.demand);
        self.request(index, frame, selected, &mut request);
        self.prune(frame, selected, &request);
        let mut group = request.drain(..).peekable();
        while let Some(first) = group.next() {
            let key = first.key;
            let mut consumer = SmallVec::new();
            consumer.push(first.consumer);
            while group.peek().is_some_and(|next| next.key == key) {
                consumer.push(group.next().unwrap().consumer);
            }
            let plan = self.catalog.input(key.input);
            if let Some(&position) = self.entry.get(&key) {
                let _scope = profile::Scope::new(profile::Phase::Replacement);
                let count = consumer.len();
                let previous = self.store[position].replace(consumer);
                self.storage = self.storage + count - previous.len();
            } else {
                let _scope = profile::Scope::new(profile::Phase::Admission);
                let Some(search) = Search::admit(crate::joining::Request {
                    input: plan,
                    index,
                    frame,
                    owner: key.owner,
                    store: &self.sharing,
                }) else {
                    continue;
                };
                let entry = Entry::new(search, consumer, self.generation);
                self.storage += entry.retained();
                let viable = entry.viable();
                let position = self.store.insert(entry);
                self.entry.insert(key, position);
                if let Some(ready) = &mut self.ready {
                    if viable {
                        ready.insert(key, position);
                    }
                } else if self.entry.len() > 32 {
                    self.ready = Some(
                        self.entry
                            .iter()
                            .filter(|&(_, &position)| self.store[position].viable())
                            .map(|(key, &position)| (key, position))
                            .collect(),
                    );
                }
                self.preparation += 1;
            }
        }
        drop(group);
        self.demand = request;
    }

    fn prune(&mut self, frame: usize, selected: Option<&Set>, request: &[Request]) {
        let _scope = profile::Scope::new(profile::Phase::Removal);
        let interval: SmallVec<[_; 4]> = selected
            .filter(|selected| selected.len() < self.entry.count(frame))
            .map_or_else(
                || smallvec::smallvec![Key::frame(frame)],
                |selected| {
                    selected
                        .iter()
                        .map(|input| Key::input(frame, input))
                        .collect()
                },
            );
        let mut expected = request.iter().peekable();
        for interval in interval {
            let removal = self.entry.extract(interval, |key, _| {
                if selected.is_some_and(|selected| !selected.contains(key.input)) {
                    return false;
                }
                while expected.peek().is_some_and(|request| request.key < *key) {
                    expected.next();
                }
                expected.peek().is_none_or(|request| request.key != *key)
            });
            for (key, position) in removal {
                if let Some(ready) = &mut self.ready {
                    ready.remove(&key);
                }
                self.storage -= self.store.remove(position).retained();
            }
        }
    }
}
