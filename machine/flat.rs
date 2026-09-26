use code::atom::Atom;
use code::output::Output;
use code::program::Program;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Pattern {
    pub(crate) input: Vec<Vec<Atom>>,
    pub(crate) output: Vec<Vec<Atom>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Flat {
    rule: Vec<Pattern>,
}

impl Flat {
    pub fn new(program: &Program) -> Option<Self> {
        if !program.scope().is_empty() {
            return None;
        }
        let rule = program
            .rule()
            .iter()
            .map(|rule| {
                Some(Pattern {
                    input: rule
                        .input()
                        .iter()
                        .map(code::particle::Particle::flat)
                        .collect::<Option<_>>()?,
                    output: rule
                        .output()
                        .iter()
                        .map(|output| match output {
                            Output::Particle(particle) => particle.flat(),
                            Output::Scope(_) => None,
                        })
                        .collect::<Option<_>>()?,
                })
            })
            .collect::<Option<_>>()?;
        Some(Self { rule })
    }

    pub(crate) fn rule(&self) -> &[Pattern] {
        &self.rule
    }
}

pub(crate) fn run(atom: &[Atom]) -> impl Iterator<Item = (Atom, usize)> + '_ {
    atom.chunk_by(|left, right| left == right)
        .map(|chunk| (chunk[0], chunk.len()))
}
