use crate::group::Permutation;
use crate::structure::{Part, Structure, Symmetry};
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
        part: vec![Part {
            role: 0,
            program,
            configuration,
        }],
        pin: Vec::new(),
    }
}

pub fn closure(generator: &mut Generator, structure: &Structure) -> Structure {
    let atom = structure.atom();
    let mut image = atom.clone();
    generator.shuffle(&mut image);
    let map = atom.iter().copied().zip(image).collect::<BTreeMap<_, _>>();
    let mut rule = structure.part[0].program.rule().to_vec();
    let mut coherence = structure.part[0].configuration.coherence().to_vec();
    let mut current = structure.clone();
    for _ in 0..generator.below(4) {
        current = current.rename(|atom| map[&atom]);
        rule.extend(current.part[0].program.rule().iter().cloned());
        coherence.extend(current.part[0].configuration.coherence().iter().cloned());
    }
    Structure {
        part: vec![Part {
            role: 0,
            program: Program::from(rule),
            configuration: Configuration::from(coherence),
        }],
        pin: Vec::new(),
    }
}

pub fn shuffle(
    generator: &mut Generator,
    structure: &Structure,
) -> (Structure, BTreeMap<Atom, Atom>) {
    let atom = structure.atom();
    let mut image = (0..u16::MAX).map(Atom).collect::<Vec<_>>();
    image.truncate(atom.len() * 3 + 1);
    generator.shuffle(&mut image);
    let map = atom.iter().copied().zip(image).collect::<BTreeMap<_, _>>();
    (structure.rename(|atom| map[&atom]), map)
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
        part: vec![Part {
            role: 0,
            program,
            configuration: Configuration::default(),
        }],
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
