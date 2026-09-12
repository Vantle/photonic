use crate::particle::Particle;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Rule<Concept> {
    pub input: Vec<Particle<Concept>>,
    pub output: Vec<Particle<Concept>>,
}

impl<Concept: Clone + Ord> Rule<Concept> {
    pub fn apply(&self, binding: &[Particle<Concept>]) -> Option<Vec<Particle<Concept>>> {
        if binding.len() != self.input.len() {
            return None;
        }
        let mut remainder = Vec::new();
        for (particle, pattern) in binding.iter().zip(&self.input) {
            remainder.extend(particle.remainder(pattern)?);
        }
        let remainder = Particle::new(remainder);
        Some(
            self.output
                .iter()
                .map(|output| output.merge(&remainder))
                .collect(),
        )
    }
}
