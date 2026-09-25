use crate::task::{Example, Task};
use code::atom::Atom;
use code::configuration::Configuration;
use code::observation::Observation;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use machine::exploration::explore;
use machine::flat::Flat;
use machine::limit::Limit;
use machine::schedule::schedule;
use machine::state::State;
use random::Generator;
use translation::vocabulary::Vocabulary;

const NAME: [&str; 6] = ["Alpha", "Bravo", "Charlie", "Echo", "Foxtrot", "Golf"];

fn particle(generator: &mut Generator, atom: usize, minimum: usize, maximum: usize) -> Particle {
    let length = minimum + generator.below(maximum - minimum + 1);
    Particle::atom(
        &(0..length)
            .map(|_| Atom(generator.below(atom) as u16))
            .collect::<Vec<_>>(),
    )
}

fn program(generator: &mut Generator, atom: usize, rule: usize) -> Program {
    Program::from(
        (0..rule)
            .map(|_| {
                let input = (0..1 + usize::from(generator.chance(0.35)))
                    .map(|_| particle(generator, atom, 1, 2))
                    .collect();
                let output = (0..generator.below(3))
                    .map(|_| Output::plain(particle(generator, atom, 0, 2)))
                    .collect();
                Rule::new(input, output)
            })
            .collect::<Vec<_>>(),
    )
}

fn configuration(generator: &mut Generator, atom: usize) -> Configuration {
    Configuration::from(
        (0..1 + generator.below(3))
            .map(|_| particle(generator, atom, 1, 3))
            .collect::<Vec<_>>(),
    )
}

fn run(program: &Flat, input: &Configuration, limit: &Limit) -> Option<(Observation, usize)> {
    let initial = State::new(input)?;
    let exploration = explore(program, initial.clone(), limit, |_| false);
    if !exploration.complete() || exploration.terminal.len() != 1 {
        return None;
    }
    let work = schedule(program, initial, limit).ok()?.work();
    Some((exploration.terminal[0].observation(), work))
}

pub fn generate(generator: &mut Generator, name: String, limit: &Limit, rule: usize) -> Task {
    loop {
        let atom = 4 + generator.below(3);
        let candidate = program(generator, atom, rule);
        let compiled = Flat::new(&candidate).expect("synthetic programs are flat");
        let mut example: Vec<Example> = Vec::new();
        let mut work = 0;
        let mut attempt = 0;
        while example.len() < 16 && attempt < 64 {
            attempt += 1;
            let input = configuration(generator, atom);
            if example.iter().any(|known| known.input == input) {
                continue;
            }
            let Some((output, effort)) = run(&compiled, &input, limit) else {
                example.clear();
                break;
            };
            work += effort;
            example.push(Example { input, output });
        }
        let changed = example
            .iter()
            .filter(|example| {
                !Observation::from(&example.input).same(&example.output, limit.individualization)
            })
            .count();
        if example.len() < 16 || changed < 8 || work < 16 {
            continue;
        }
        let holdout = example.split_off(10);
        return Task {
            name,
            vocabulary: Vocabulary::try_from(
                NAME[..atom]
                    .iter()
                    .map(|name| (*name).to_owned())
                    .collect::<Vec<_>>(),
            )
            .expect("the synthetic names fit a vocabulary"),
            example,
            holdout,
            reference: Some(candidate),
            goal: None,
        };
    }
}
