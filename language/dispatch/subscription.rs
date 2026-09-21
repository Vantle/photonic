use super::entry::Entry;
use super::{Key, Network};
use crate::index::Index;
use crate::membership::Set;
use crate::replay::Search;
use smallvec::SmallVec;

impl Network {
    pub(super) fn frame(&mut self, index: &Index, frame: usize, selected: Option<&Set>) {
        #[cfg(feature = "measurement")]
        let _measurement = crate::measurement::profile::Scope::new(
            crate::measurement::profile::Phase::Subscription,
        );
        let request = self.request(index, frame, selected);
        #[cfg(feature = "measurement")]
        let measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Removal);
        let interval = selected
            .filter(|selected| selected.len() < self.entry.count(frame))
            .map_or_else(
                || vec![Key::frame(frame)],
                |selected| {
                    selected
                        .iter()
                        .map(|&input| Key::input(frame, input))
                        .collect()
                },
            );
        let mut expected = request.iter().peekable();
        for interval in interval {
            let removal = self.entry.extract(interval, |key, _| {
                if selected.is_some_and(|selected| !selected.contains(&key.input)) {
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
        #[cfg(feature = "measurement")]
        drop(measurement);
        let mut request = request.into_iter().peekable();
        while let Some(first) = request.next() {
            let key = first.key;
            let mut consumer = SmallVec::new();
            consumer.push(first.consumer);
            while request.peek().is_some_and(|next| next.key == key) {
                consumer.push(request.next().unwrap().consumer);
            }
            let plan = self.catalog.input(key.input);
            if let Some(&position) = self.entry.get(&key) {
                #[cfg(feature = "measurement")]
                let _measurement = crate::measurement::profile::Scope::new(
                    crate::measurement::profile::Phase::Replacement,
                );
                let entry = &mut self.store[position];
                self.storage -= entry.retained();
                entry.replace(consumer);
                self.storage += entry.retained();
            } else {
                #[cfg(feature = "measurement")]
                let _measurement = crate::measurement::profile::Scope::new(
                    crate::measurement::profile::Phase::Admission,
                );
                let entry = Entry::new(
                    Search::planned(crate::joining::Request {
                        input: plan,
                        index,
                        frame,
                        owner: key.owner,
                        store: &self.sharing,
                    }),
                    consumer,
                    self.generation,
                );
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
                            .map(|(&key, &position)| (key, position))
                            .collect(),
                    );
                }
                self.preparation += 1;
            }
        }
    }
}
