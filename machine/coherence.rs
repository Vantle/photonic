use crate::flat::run;
use code::atom::Atom;
use code::particle::Particle;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Token {
    pub atom: Atom,
    pub id: u32,
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Coherence {
    token: Vec<Token>,
}

impl Coherence {
    pub fn new(mut token: Vec<Token>) -> Self {
        token.sort_unstable();
        Self { token }
    }

    pub fn token(&self) -> &[Token] {
        &self.token
    }

    pub fn len(&self) -> usize {
        self.token.len()
    }

    pub fn range(&self, atom: Atom) -> &[Token] {
        let start = self.token.partition_point(|token| token.atom < atom);
        let end = self.token.partition_point(|token| token.atom <= atom);
        &self.token[start..end]
    }

    pub fn particle(&self) -> Particle {
        Particle::atom(
            &self
                .token
                .iter()
                .map(|token| token.atom)
                .collect::<Vec<_>>(),
        )
    }

    pub fn contains(&self, pattern: &[Atom]) -> bool {
        run(pattern).all(|(atom, count)| self.range(atom).len() >= count)
    }
}
