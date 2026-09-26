use crate::vocabulary::Vocabulary;
use code::atom::Atom;
use code::configuration::Configuration;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::scope::Scope;
use code::value::Value;
use random::Generator;

pub fn vocabulary() -> Vocabulary {
    Vocabulary::try_from(["A", "B", "C", "D", "X"].map(str::to_owned).to_vec()).unwrap()
}

struct Shape {
    nesting: f64,
    depth: usize,
}

fn value(generator: &mut Generator, shape: &Shape, depth: usize) -> Value {
    if depth < shape.depth && generator.chance(shape.nesting) {
        return Value::Rule(Box::new(rule(generator, shape, depth + 1)));
    }
    Value::Atom(Atom(generator.below(5) as u16))
}

fn particle(
    generator: &mut Generator,
    shape: &Shape,
    depth: usize,
    minimum: usize,
    maximum: usize,
) -> Particle {
    let length = minimum + generator.below(maximum - minimum + 1);
    Particle::from(
        (0..length)
            .map(|_| value(generator, shape, depth))
            .collect::<Vec<_>>(),
    )
}

fn rule(generator: &mut Generator, shape: &Shape, depth: usize) -> Rule {
    let input = (0..1 + generator.below(2))
        .map(|_| {
            let minimum = usize::from(!generator.chance(0.1));
            particle(generator, shape, depth, minimum, 2)
        })
        .collect();
    let output = (0..generator.below(3))
        .flat_map(|_| {
            if depth < shape.depth && generator.chance(shape.nesting) {
                return group(generator, shape, depth);
            }
            vec![Output::Particle(particle(generator, shape, depth, 0, 2))]
        })
        .collect();
    Rule::new(input, output)
}

fn group(generator: &mut Generator, shape: &Shape, depth: usize) -> Vec<Output> {
    let mut member = (0..generator.below(3))
        .map(|_| Output::Particle(particle(generator, shape, depth, 0, 2)))
        .collect::<Vec<_>>();
    if depth + 1 < shape.depth && generator.chance(shape.nesting) {
        member.extend(group(generator, shape, depth + 1));
    }
    let rule = (0..1 + generator.below(2))
        .map(|_| rule(generator, shape, depth + 1))
        .collect();
    Output::group(member, rule)
}

fn scope(generator: &mut Generator, shape: &Shape) -> Vec<Scope> {
    if !generator.chance(shape.nesting) {
        return Vec::new();
    }
    group(generator, shape, 0)
        .into_iter()
        .filter_map(|output| match output {
            Output::Scope(scope) => Some(scope),
            Output::Particle(_) => None,
        })
        .collect()
}

fn program(generator: &mut Generator, shape: &Shape) -> Program {
    let rule = (0..1 + generator.below(3))
        .map(|_| rule(generator, shape, 0))
        .collect::<Vec<_>>();
    Program::new(rule, scope(generator, shape))
}

pub fn flat(generator: &mut Generator) -> Program {
    program(
        generator,
        &Shape {
            nesting: 0.0,
            depth: 0,
        },
    )
}

pub fn nested(generator: &mut Generator) -> Program {
    program(
        generator,
        &Shape {
            nesting: 0.2,
            depth: 2,
        },
    )
}

pub fn configuration(generator: &mut Generator) -> Configuration {
    Configuration::from(
        (0..1 + generator.below(3))
            .map(|_| {
                particle(
                    generator,
                    &Shape {
                        nesting: 0.0,
                        depth: 0,
                    },
                    0,
                    1,
                    3,
                )
            })
            .collect::<Vec<_>>(),
    )
}
