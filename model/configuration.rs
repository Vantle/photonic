use crate::context::{Frame, Identity};
use crate::failure::Failure;
use crate::history::History;
use crate::occurrence::{self, Occurrence};
use crate::world::{self, World};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Configuration {
    pub(crate) root: Identity,
    pub(crate) world: BTreeMap<world::Identity, World>,
    pub(crate) frame: BTreeMap<Identity, Frame>,
    pub(crate) history: History,
    pub(crate) allocation: crate::allocation::Allocation,
}

fn occurrence<'a>(
    value: &'a [Occurrence],
    history: &History,
    registry: &mut BTreeMap<occurrence::Identity, &'a Occurrence>,
    context: &mut BTreeSet<Identity>,
) -> Result<(), Failure> {
    let mut present = BTreeSet::new();
    for occurrence in value {
        if !present.insert(occurrence.identity) {
            return Err(Failure::Repeated(occurrence.identity));
        }
        history.permits(&occurrence.history)?;
        if registry
            .insert(occurrence.identity, occurrence)
            .is_some_and(|previous| previous != occurrence)
        {
            return Err(Failure::Identity(occurrence.identity));
        }
        occurrence.value.context(context);
    }
    Ok(())
}

impl Configuration {
    pub fn new(
        root: Identity,
        world: Vec<World>,
        frame: Vec<Frame>,
        history: History,
    ) -> Result<Self, Failure> {
        let mut registry = BTreeMap::new();
        let mut context = BTreeSet::new();
        context.insert(root);
        let mut identity = BTreeSet::new();
        for frame in &frame {
            if !identity.insert(frame.identity) {
                return Err(Failure::Context(frame.identity));
            }
            context.extend(frame.parent);
            context.extend(frame.lexical);
            for rule in &frame.declaration {
                rule.collect(&mut context);
            }
            occurrence(&frame.held, &history, &mut registry, &mut context)?;
        }
        let mut unique = BTreeSet::new();
        for world in &world {
            if !unique.insert(world.identity) {
                return Err(Failure::World(world.identity));
            }
            context.insert(world.context);
            occurrence(&world.occurrence, &history, &mut registry, &mut context)?;
        }
        if let Some(&missing) = context.difference(&identity).next() {
            return Err(Failure::Context(missing));
        }
        let allocation = crate::allocation::Allocation::new(
            registry.keys().map(|identity| identity.0),
            frame.iter().map(|frame| frame.identity.0),
            world.iter().map(|world| world.identity.0),
        );
        Ok(Self {
            root,
            world: world
                .into_iter()
                .map(|world| (world.identity, world))
                .collect(),
            frame: frame
                .into_iter()
                .map(|frame| (frame.identity, frame))
                .collect(),
            history,
            allocation,
        })
    }

    pub fn world(&self) -> impl Iterator<Item = &World> {
        self.world.values()
    }

    pub fn frame(&self) -> impl Iterator<Item = &Frame> {
        self.frame.values()
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn root(&self) -> Identity {
        self.root
    }

    pub(crate) fn reclaim(&mut self) {
        let mut pending = BTreeSet::from([self.root]);
        for world in self.world.values() {
            pending.insert(world.context);
            for occurrence in &world.occurrence {
                occurrence.value.context(&mut pending);
            }
        }
        let mut retained = BTreeSet::new();
        while let Some(identity) = pending.pop_first() {
            if !retained.insert(identity) {
                continue;
            }
            let frame = &self.frame[&identity];
            pending.extend(frame.parent);
            pending.extend(frame.lexical);
            for rule in &frame.declaration {
                rule.collect(&mut pending);
            }
            for occurrence in &frame.held {
                occurrence.value.context(&mut pending);
            }
        }
        self.frame.retain(|identity, _| retained.contains(identity));
    }

    pub(crate) fn visible(&self, site: Identity, owner: Identity) -> bool {
        let mut cursor = Some(site);
        let mut visited = BTreeSet::new();
        while let Some(identity) = cursor {
            if identity == owner {
                return true;
            }
            if !visited.insert(identity) {
                return false;
            }
            cursor = self.frame[&identity].lexical;
        }
        false
    }
}
