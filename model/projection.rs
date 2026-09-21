use crate::activation;
use crate::application::{self, Code, Request};
use crate::context;
use crate::failure::Failure;
use crate::flow::Place;
use crate::path::Path;
use crate::structure::{Input, Particle};
use crate::world;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    pub frame: context::Identity,
    pub owner: Option<context::Identity>,
    pub world: BTreeSet<world::Identity>,
    pub footprint: BTreeSet<Place>,
    pub exact: BTreeSet<Place>,
    pub read: BTreeSet<Place>,
}

#[derive(Debug)]
pub struct Projection<'a> {
    path: &'a Path,
    request: Request,
    binding: Binding,
}

impl Projection<'_> {
    pub fn path(&self) -> &Path {
        self.path
    }
    pub fn request(&self) -> &Request {
        &self.request
    }
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
}

fn input(path: &Path, input: &Input) -> Result<Input, Failure> {
    let context = |source| {
        path.flow()
            .frame
            .iter()
            .find_map(|(&target, &origin)| (origin == Some(source)).then_some(target))
            .ok_or(Failure::Origin(source))
    };
    Ok(Input::new(
        input
            .particle()
            .iter()
            .map(|particle| {
                Ok(Particle::new(
                    particle
                        .value()
                        .iter()
                        .cloned()
                        .map(|value| activation::rename(value, &context))
                        .collect::<Result<_, _>>()?,
                ))
            })
            .collect::<Result<_, Failure>>()?,
    ))
}

pub fn project<'a>(path: &'a Path, request: Request) -> Result<Projection<'a>, Failure> {
    let source = path.source();
    let target = path.target();
    let flow = path.flow();
    let (site, selected) = application::select(target, &request)?;
    let frame = flow.frame[&site].ok_or(Failure::Origin(site))?;
    let (input, owner, read) = match request.code {
        Code::Local { .. } => {
            let (rule, owner, read) = application::code(target, request.code, site, &selected)?;
            (
                rule.input.clone(),
                flow.frame[&owner],
                flow.project(&read).map_err(Failure::Flow)?,
            )
        }
        Code::Declaration { context, position } => {
            if !source.visible(frame, context) {
                return Err(Failure::Context(context));
            }
            let rule = source.frame[&context]
                .declaration
                .get(position)
                .ok_or(Failure::Declaration { context, position })?;
            (input(path, &rule.input)?, Some(context), BTreeSet::new())
        }
    };
    let consumed = application::binding(target, &request, &input)?;
    let footprint = flow.project(&consumed).map_err(Failure::Flow)?;
    let mut world = selected
        .iter()
        .flat_map(|identity| flow.context[identity].iter().copied())
        .collect::<BTreeSet<_>>();
    for &place in &footprint {
        match place {
            Place::World(identity, _) => {
                world.insert(identity);
            }
            Place::Held(_, _) => return Err(Failure::Held(place)),
        }
    }
    for identity in &world {
        if source.world[identity].context != frame {
            return Err(Failure::Context(source.world[identity].context));
        }
    }
    let mut exact = BTreeSet::new();
    for place in consumed {
        let basis = &flow.resource[&place];
        if basis.len() != 1 {
            continue;
        }
        let Some(&origin @ Place::World(world, identity)) = basis.first() else {
            continue;
        };
        let original = source.world[&world]
            .occurrence
            .iter()
            .find(|value| value.identity == identity)
            .unwrap();
        let Place::World(world, identity) = place else {
            unreachable!()
        };
        let value = target.world[&world]
            .occurrence
            .iter()
            .find(|value| value.identity == identity)
            .unwrap();
        let mapped = activation::rename(value.value.clone(), &|context| {
            flow.frame[&context].ok_or(Failure::Origin(context))
        });
        if mapped.is_ok_and(|value| value == original.value) {
            exact.insert(origin);
        }
    }
    Ok(Projection {
        path,
        request,
        binding: Binding {
            frame,
            owner,
            world,
            footprint,
            exact,
            read,
        },
    })
}
