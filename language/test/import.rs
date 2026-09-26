use crate::application::{self, Request};
use crate::basis::Set;
use crate::flow::{Binding, Closure, Flow};
use crate::place::Place;
use crate::program::{Instruction, Output, Symbol};
use crate::state::{Frame, State, Token, World};

#[test]
fn coalescence() {
    let frame = Frame {
        scope: 0,
        parent: None,
        lexical: None,
        particle: Default::default(),
        held: Vec::new(),
    };
    let atom = Token {
        id: 0,
        value: Symbol::Atom(0),
        capture: None,
    };
    let source = State {
        world: vec![
            World {
                frame: 0,
                particle: vec![
                    atom.clone(),
                    Token {
                        id: 1,
                        value: Symbol::Atom(1),
                        capture: None,
                    },
                ],
            }
            .into(),
            World {
                frame: 0,
                particle: vec![atom.clone()],
            }
            .into(),
        ]
        .into(),
        frame: vec![frame.clone().into()].into(),
    };
    let endpoint = State {
        world: Default::default(),
        frame: vec![
            frame.into(),
            Frame {
                scope: 1,
                parent: Some(0),
                lexical: Some(0),
                particle: Default::default(),
                held: vec![Token { id: 10, ..atom }, Token { id: 11, ..atom }],
            }
            .into(),
        ]
        .into(),
    };
    let flow = Flow {
        resource: [
            (Place::Held(1, 10), Set::single(Place::World(0, 0))),
            (Place::Held(1, 11), Set::single(Place::World(1, 0))),
        ]
        .into_iter()
        .collect(),
        context: vec![],
        frame: vec![Some(0), None],
    };
    let rule = Instruction {
        output: vec![Output::Particle(vec![Symbol::Rule(0)])],
        ..Default::default()
    };
    let binding = Binding {
        world: [0, 1].into_iter().collect(),
        footprint: Set::single(Place::World(0, 1)),
        exact: Set::single(Place::World(0, 1)),
        read: Set::default(),
    };
    let result = application::apply(Request {
        scope: &[],
        source: &source,
        frame: 0,
        owner: crate::application::Owner::Capture(Closure {
            state: &endpoint,
            flow: &flow,
            capture: 1,
        }),
        rule: &rule,
        binding: &binding,
    });
    assert_eq!(result.state.frame[1].held, vec![atom]);
    assert_eq!(
        result.flow.resource[&Place::Held(1, 0)],
        [Place::World(0, 0), Place::World(1, 0)]
            .into_iter()
            .collect()
    );
    assert_eq!(result.state.world[0].particle.len(), 2);
    assert_eq!(result.state.world[0].particle[0].id, 0);
    assert_eq!(result.state.world[0].particle[1].capture, Some(1));
    assert!(source.frame[0].held.is_empty());
}
