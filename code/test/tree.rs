use super::support::{A, B, C, D, nested, particle, rule};
use crate::output::Output;
use crate::particle::Particle;
use crate::program::Program;
use crate::rule::Rule;
use crate::tree::{Place, transform, walk};
use crate::value::Value;

fn sample() -> Program {
    let inner = rule(&[&[C]], &[&[D]]);
    let carrier = Rule::new(
        vec![particle(&[A])],
        vec![Output::plain(Particle::from(vec![
            Value::Atom(crate::atom::Atom(B)),
            nested(inner.clone()),
        ]))],
    );
    let scope = Rule::new(
        vec![particle(&[B])],
        vec![Output::new(particle(&[]), Some(vec![inner]))],
    );
    Program::from(vec![carrier, scope])
}

#[test]
fn order() {
    let program = sample();
    let node = walk(&program);
    assert_eq!(node.len(), 4);
    assert_eq!(node[0].depth, 0);
    assert_eq!(node[0].place, Place::Program);
    assert_eq!(node[1].depth, 1);
    assert_eq!(node[1].place, Place::Value { parent: 0 });
    assert_eq!(node[2].place, Place::Program);
    assert_eq!(node[3].place, Place::Body { parent: 2 });
    assert_eq!(node[1].rule, node[3].rule);
}

#[test]
fn rewrite() {
    let program = sample();
    let deleted = transform(&program, 1, |_| Vec::new());
    assert_eq!(walk(&deleted).len(), 3);
    assert!(deleted.rule()[0].output()[0].particle().flat().is_some());
    let doubled = transform(&program, 3, |rule| vec![rule.clone(), rule.clone()]);
    assert_eq!(doubled.rule()[1].output()[0].body().unwrap().len(), 2);
    let emptied = transform(&program, 3, |_| Vec::new());
    assert!(emptied.rule()[1].output()[0].body().is_none());
    let replaced = transform(&program, 0, |_| vec![rule(&[&[D]], &[])]);
    assert_eq!(walk(&replaced).len(), 3);
    assert!(replaced.rule().iter().any(|rule| rule.output().is_empty()));
    assert_eq!(transform(&program, 9, |_| Vec::new()), program);
}

#[test]
fn flatness() {
    assert!(!sample().flat());
    assert!(Program::from(vec![rule(&[&[A]], &[&[B]])]).flat());
}
