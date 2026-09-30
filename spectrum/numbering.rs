use crate::configuration::Configuration;
use crate::exploration::{self, Event};
use photonic::snapshot::Snapshot;
use std::collections::VecDeque;

// Each engine numbers configurations and events in the order it finds them. An exploration of every
// future is renumbered by what both engines share: configurations by their distance from the start
// along supported events, then along any events, then by their canonical form; events by their
// source, target, rule and binding. So both engines' answers name the same handles, and the order
// of work inside an engine never changes one.
pub struct Numbering {
    rank: Vec<usize>,
    origin: Vec<usize>,
}

fn distance(count: usize, event: &[Event], admit: impl Fn(&Event) -> bool) -> Vec<usize> {
    let mut outgoing = vec![Vec::new(); count];
    for value in event.iter().filter(|value| admit(value)) {
        outgoing[value.source].push(value.target);
    }
    let mut distance = vec![usize::MAX; count];
    if count == 0 {
        return distance;
    }
    distance[0] = 0;
    let mut queue = VecDeque::from([0]);
    while let Some(current) = queue.pop_front() {
        for &next in &outgoing[current] {
            if distance[next] == usize::MAX {
                distance[next] = distance[current] + 1;
                queue.push_back(next);
            }
        }
    }
    distance
}

impl Numbering {
    pub(crate) fn new(configuration: &[Configuration], event: &[Event]) -> Self {
        let count = configuration.len();
        let supported = distance(count, event, |value| value.supported);
        let any = distance(count, event, |_| true);
        let mut order = (0..count).collect::<Vec<_>>();
        order.sort_by(|&left, &right| {
            (supported[left], any[left], &configuration[left]).cmp(&(
                supported[right],
                any[right],
                &configuration[right],
            ))
        });
        let mut rank = vec![0; count];
        for (position, &index) in order.iter().enumerate() {
            rank[index] = position;
        }
        let mut origin = (0..event.len()).collect::<Vec<_>>();
        origin.sort_by_key(|&index| {
            let value = &event[index];
            (
                rank[value.source],
                rank[value.target],
                value.rule,
                &value.world,
                &value.footprint,
                &value.exact,
                &value.read,
            )
        });
        Self { rank, origin }
    }

    pub(crate) fn apply(
        &self,
        configuration: Vec<Configuration>,
        event: Vec<Event>,
    ) -> (Vec<Configuration>, Vec<Event>) {
        let mut placed = configuration
            .into_iter()
            .zip(&self.rank)
            .map(|(value, &rank)| (rank, value))
            .collect::<Vec<_>>();
        placed.sort_unstable_by_key(|&(rank, _)| rank);
        let mut position = vec![0; self.origin.len()];
        for (canonical, &index) in self.origin.iter().enumerate() {
            position[index] = canonical;
        }
        let mut slot = event.into_iter().map(Some).collect::<Vec<_>>();
        let event = self
            .origin
            .iter()
            .map(|&index| {
                let value = slot[index].take().expect("every event is placed once");
                Event {
                    source: self.rank[value.source],
                    target: self.rank[value.target],
                    deduction: value.deduction.iter().map(|&step| position[step]).collect(),
                    ..value
                }
            })
            .collect();
        (placed.into_iter().map(|(_, value)| value).collect(), event)
    }

    pub fn configuration(&self, index: usize) -> usize {
        self.rank[index]
    }

    pub fn event(&self, index: usize) -> usize {
        self.origin[index]
    }
}

impl From<&Snapshot> for Numbering {
    fn from(snapshot: &Snapshot) -> Self {
        let configuration = snapshot
            .state
            .iter()
            .map(exploration::configuration)
            .collect::<Vec<_>>();
        Self::new(&configuration, &exploration::event(snapshot))
    }
}
