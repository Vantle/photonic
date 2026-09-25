use crate::cause::Role;
use crate::exploration::{self, Event, Exploration};
use crate::failure::{Code, Failure};
use crate::recording::Mode;
use photonic::place::Place;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Line {
    pub configuration: usize,
    pub occurrence: usize,
    pub event: Option<usize>,
    pub role: Role,
    pub source: Vec<Place>,
}

fn place(exploration: &Exploration, configuration: usize, id: usize) -> Option<Place> {
    let entry = &exploration.configuration[configuration];
    let world = entry
        .coherence
        .iter()
        .position(|coherence| {
            coherence
                .occurrence
                .iter()
                .any(|occurrence| occurrence.id == id)
        })
        .map(|index| Place::World(index, id));
    let frame = entry.frame.iter().enumerate().find_map(|(index, frame)| {
        if frame.rule.iter().any(|occurrence| occurrence.id == id) {
            return Some(Place::Context(index, id));
        }
        frame
            .held
            .iter()
            .any(|occurrence| occurrence.id == id)
            .then_some(Place::Held(index, id))
    });
    world.or(frame)
}

fn nested(exploration: &Exploration, event: &Event) -> bool {
    exploration.rule[event.rule]
        .definition
        .output
        .iter()
        .any(|output| output.body.is_some())
}

fn same(exploration: &Exploration, event: &Event, target: Place, source: Place) -> bool {
    let value = |configuration, place| {
        exploration
            .find(configuration, exploration::id(place))
            .map(|entry| &entry.value)
    };
    let after = value(event.target, target);
    after.is_some() && after == value(event.source, source)
}

fn role(exploration: &Exploration, event: &Event, target: Place, source: &[Place]) -> Role {
    let [place] = source else {
        return Role::Produced;
    };
    if !event.footprint.contains(place) {
        return match place {
            Place::World(world, _) if event.world.contains(world) => Role::Remainder,
            _ => Role::Untouched,
        };
    }
    if matches!(target, Place::Held(..)) {
        return Role::Held;
    }
    let witness = !event.exact.contains(place)
        && nested(exploration, event)
        && same(exploration, event, target, *place);
    if witness {
        return Role::Witness;
    }
    Role::Produced
}

pub fn origin(exploration: &Exploration, event: usize, target: Place) -> (Role, Vec<Place>) {
    let entry = &exploration.event[event];
    let source = entry
        .resource
        .iter()
        .filter(|link| link.target == target)
        .flat_map(|link| link.source.iter().copied())
        .collect::<Vec<_>>();
    (role(exploration, entry, target, &source), source)
}

pub fn lineage(
    exploration: &Exploration,
    configuration: usize,
    occurrence: usize,
) -> Result<Vec<Line>, Failure> {
    if exploration.mode == Mode::Path {
        return Err(Failure::new(
            Code::Exploration,
            "lineage follows each event's place map, which exhaustive explorations record; explore without path mode",
        ));
    }
    if configuration != 0 && exploration.parent[configuration].is_none() {
        return Err(Failure::new(
            Code::Handle,
            format!("s{configuration} has no grounded path, so its occurrences have no lineage"),
        ));
    }
    let mut line = Vec::new();
    let mut current = (configuration, occurrence);
    loop {
        let (node, id) = current;
        let Some(event) = exploration.parent[node] else {
            line.push(Line {
                configuration: node,
                occurrence: id,
                event: None,
                role: Role::Initial,
                source: Vec::new(),
            });
            return Ok(line);
        };
        let target = place(exploration, node, id).ok_or_else(|| {
            Failure::new(
                Code::Handle,
                format!("s{node}.o{id} names no occurrence in s{node}"),
            )
        })?;
        let (role, source) = origin(exploration, event, target);
        let next = match (role, source.as_slice()) {
            (Role::Produced, _) | (_, []) => None,
            (_, [first, ..]) => Some(exploration::id(*first)),
        };
        line.push(Line {
            configuration: node,
            occurrence: id,
            event: Some(event),
            role,
            source,
        });
        let Some(next) = next else {
            return Ok(line);
        };
        current = (exploration.event[event].source, next);
    }
}
