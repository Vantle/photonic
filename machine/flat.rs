use code::atom::Atom;
use code::program::Program;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pattern {
    pub input: Vec<Vec<Atom>>,
    pub output: Vec<Vec<Atom>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Flat {
    rule: Vec<Pattern>,
}

impl Flat {
    pub fn new(program: &Program) -> Option<Self> {
        let rule = program
            .rule()
            .iter()
            .map(|rule| {
                if rule.output().iter().any(|output| output.body().is_some()) {
                    return None;
                }
                Some(Pattern {
                    input: rule
                        .input()
                        .iter()
                        .map(code::particle::Particle::flat)
                        .collect::<Option<_>>()?,
                    output: rule
                        .output()
                        .iter()
                        .map(|output| output.particle().flat())
                        .collect::<Option<_>>()?,
                })
            })
            .collect::<Option<_>>()?;
        Some(Self { rule })
    }

    pub fn rule(&self) -> &[Pattern] {
        &self.rule
    }
}

pub fn run(atom: &[Atom]) -> impl Iterator<Item = (Atom, usize)> + '_ {
    atom.chunk_by(|left, right| left == right)
        .map(|chunk| (chunk[0], chunk.len()))
}
