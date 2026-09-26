use super::support::{A, B, C, D, nested, particle, rule};
use crate::output::Output;
use crate::particle::Particle;
use crate::program::Program;
use crate::rule::Rule;

#[test]
fn flatness() {
    let inner = rule(&[&[C]], &[&[D]]);
    let valued = Rule::new(
        vec![particle(&[A])],
        vec![Output::plain(Particle::from(vec![nested(inner.clone())]))],
    );
    let scoped = Rule::new(
        vec![particle(&[B])],
        vec![Output::new(particle(&[]), Some(vec![inner]))],
    );
    assert!(!Program::from(vec![valued]).flat());
    assert!(!Program::from(vec![scoped]).flat());
    assert!(Program::from(vec![rule(&[&[A]], &[&[B]])]).flat());
}
