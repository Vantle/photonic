use crate::group::Permutation;
use crate::structure::{Part, Structure, Symmetry, image};
use code::atom::Atom;
use code::configuration::Configuration;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::value::Value;
use random::Generator;
use std::collections::BTreeMap;

pub struct Shape {
    pub atom: usize,
    pub rule: usize,
    pub depth: usize,
}

pub fn particle(generator: &mut Generator, shape: &Shape, depth: usize) -> Particle {
    let length = generator.below(4);
    Particle::from(
        (0..length)
            .map(|_| {
                if depth < shape.depth && generator.chance(0.15) {
                    return Value::Rule(Box::new(rule(generator, shape, depth + 1)));
                }
                Value::Atom(Atom(generator.below(shape.atom) as u16))
            })
            .collect::<Vec<_>>(),
    )
}

pub fn rule(generator: &mut Generator, shape: &Shape, depth: usize) -> Rule {
    let input = (0..1 + generator.below(2))
        .map(|_| particle(generator, shape, depth))
        .collect();
    let output = (0..generator.below(3))
        .map(|_| {
            let body = (depth < shape.depth && generator.chance(0.1))
                .then(|| vec![rule(generator, shape, depth + 1)]);
            Output::new(particle(generator, shape, depth), body)
        })
        .collect();
    Rule::new(input, output)
}

pub fn structure(generator: &mut Generator, shape: &Shape) -> Structure {
    let program = Program::from(
        (0..shape.rule)
            .map(|_| rule(generator, shape, 0))
            .collect::<Vec<_>>(),
    );
    let configuration = Configuration::from(
        (0..generator.below(3))
            .map(|_| particle(generator, shape, 0))
            .collect::<Vec<_>>(),
    );
    Structure {
        program: Part {
            program,
            configuration,
        },
        target: None,
        pin: Vec::new(),
    }
}

pub fn rename(structure: &Structure, map: impl Fn(Atom) -> Atom) -> Structure {
    Structure {
        program: image(&structure.program, &map),
        target: structure.target.as_ref().map(|target| image(target, &map)),
        pin: structure.pin.iter().map(|&atom| map(atom)).collect(),
    }
}

fn free(structure: &Structure) -> Vec<Atom> {
    structure
        .atom()
        .into_iter()
        .filter(|atom| !structure.pin.contains(atom))
        .collect()
}

fn join(left: &Part, right: &Part) -> Part {
    Part {
        program: Program::from([left.program.rule(), right.program.rule()].concat()),
        configuration: Configuration::from(
            [
                left.configuration.coherence(),
                right.configuration.coherence(),
            ]
            .concat(),
        ),
    }
}

pub fn closure(generator: &mut Generator, structure: &Structure) -> Structure {
    let atom = free(structure);
    let mut order = atom.clone();
    generator.shuffle(&mut order);
    let map = atom.into_iter().zip(order).collect::<BTreeMap<_, _>>();
    let mut current = structure.clone();
    let mut result = structure.clone();
    for _ in 0..generator.below(4) {
        current = rename(&current, |atom| map.get(&atom).copied().unwrap_or(atom));
        result = Structure {
            program: join(&result.program, &current.program),
            target: result
                .target
                .as_ref()
                .zip(current.target.as_ref())
                .map(|(own, copy)| join(own, copy)),
            pin: result.pin,
        };
    }
    result
}

pub fn shuffle(generator: &mut Generator, structure: &Structure) -> Structure {
    let atom = free(structure);
    let mut fresh = (0..u16::MAX)
        .map(Atom)
        .filter(|atom| !structure.pin.contains(atom))
        .take(atom.len() * 3 + 1)
        .collect::<Vec<_>>();
    generator.shuffle(&mut fresh);
    let map = atom.into_iter().zip(fresh).collect::<BTreeMap<_, _>>();
    rename(structure, |atom| map.get(&atom).copied().unwrap_or(atom))
}

pub fn permutation(atom: &[Atom]) -> Vec<Vec<Atom>> {
    if atom.is_empty() {
        return vec![Vec::new()];
    }
    let mut result = Vec::new();
    for index in 0..atom.len() {
        let mut rest = atom.to_vec();
        let first = rest.remove(index);
        for mut tail in permutation(&rest) {
            tail.insert(0, first);
            result.push(tail);
        }
    }
    result
}

pub struct Written<'text>(pub Vec<Vec<&'text str>>, pub Vec<Vec<&'text str>>);

pub fn named(program: &[Written<'_>], name: &mut Vec<String>) -> Program {
    let mut atom = |text: &str| {
        let position = name
            .iter()
            .position(|entry| entry == text)
            .unwrap_or_else(|| {
                name.push(text.to_owned());
                name.len() - 1
            });
        Atom(position as u16)
    };
    let mut particle = |entry: &Vec<&str>| {
        Particle::from(
            entry
                .iter()
                .map(|text| Value::Atom(atom(text)))
                .collect::<Vec<_>>(),
        )
    };
    Program::from(
        program
            .iter()
            .map(|Written(input, output)| {
                Rule::new(
                    input.iter().map(&mut particle).collect(),
                    output
                        .iter()
                        .map(|entry| Output::plain(particle(entry)))
                        .collect(),
                )
            })
            .collect::<Vec<_>>(),
    )
}

pub fn single(program: Program) -> Structure {
    Structure {
        program: Part {
            program,
            configuration: Configuration::default(),
        },
        target: None,
        pin: Vec::new(),
    }
}

pub fn generator(symmetry: &Symmetry) -> Vec<Permutation> {
    let mut generator = symmetry.generator.clone();
    for block in &symmetry.block {
        generator.push(Permutation::new([
            (block[0], block[1]),
            (block[1], block[0]),
        ]));
        generator.push(Permutation::new(
            block
                .iter()
                .copied()
                .zip(block.iter().copied().cycle().skip(1)),
        ));
    }
    generator
}
