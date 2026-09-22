use super::Structure;

#[test]
fn equivalence() {
    for source in [
        "A.X,B.X [A,B] C,D",
        "A [A] (B [B] C) [C] (D [D] A)",
        "Seed.A.X [Seed] ([A] B) ([A] C)",
        "A.([A] B),A.([A] C) [B,C] D",
        "A,A,B [A,B] C [A,C] D",
    ] {
        let mut runtime = crate::runtime::Runtime::new(crate::lowering::parse(source).unwrap());
        runtime.run(100_000, None);
        let mut structure = Structure::default();
        for state in runtime.state.iter().chain(runtime.state.iter().rev()) {
            let expected = crate::fingerprint::signature(state);
            assert_eq!(structure.advance(state), expected, "{source} {state:?}");
            assert_eq!(structure.advance(state), expected, "unchanged {source}");
            let world = (0..state.world.len()).rev().collect::<Vec<_>>();
            let frame = std::iter::once(0)
                .chain((1..state.frame.len()).rev())
                .collect::<Vec<_>>();
            let renamed = state.rename(&world, &frame).state;
            assert_eq!(structure.advance(&renamed), expected, "renamed {source}");
        }
    }
}

#[test]
fn multiplicity() {
    let program = crate::program::Program::new(crate::lowering::parse("A").unwrap());
    let mut state = crate::state::State::initial(&program);
    let mut structure = Structure::default();
    assert_eq!(
        structure.advance(&state),
        crate::fingerprint::signature(&state)
    );
    state.world.push(state.world[0].clone());
    assert_eq!(
        structure.advance(&state),
        crate::fingerprint::signature(&state)
    );
    state.world.pop();
    assert_eq!(
        structure.advance(&state),
        crate::fingerprint::signature(&state)
    );
}

#[test]
fn mutation() {
    use crate::program::Symbol;
    use crate::state::{Frame, State, Token, World};
    use std::sync::Arc;
    let mut structure = Structure::default();
    for seed in (0..512usize).chain((0..512usize).rev()) {
        let token = |id| Token {
            id,
            value: if id == 0 {
                Symbol::Rule(0)
            } else {
                Symbol::Atom(id % 3)
            },
            capture: (id == 0).then_some(seed % 3),
        };
        let state = State {
            frame: (0..3)
                .map(|index| {
                    Arc::new(Frame {
                        scope: (seed + index) % 3,
                        parent: (index > 0).then_some(0),
                        lexical: (index > 0).then_some(0),
                        particle: Default::default(),
                        held: if seed & (1 << index) != 0 {
                            vec![token(index)]
                        } else {
                            Vec::new()
                        },
                    })
                })
                .collect(),
            world: (0..6)
                .filter(|&index| seed & (1 << index) != 0)
                .map(|index| {
                    Arc::new(World {
                        frame: index % 3,
                        particle: (0..=(seed + index) % 3)
                            .map(|position| token((index + position) % 4))
                            .collect(),
                    })
                })
                .collect(),
        };
        assert_eq!(
            structure.advance(&state),
            crate::fingerprint::signature(&state),
            "{seed}"
        );
    }
}
