use super::support::{A, B, C, D, nested, particle, rule};
use crate::tree::{Place, transform, walk};
use code::atom::Atom;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::scope::Scope;
use code::value::Value;

fn sample() -> Program {
    let inner = rule(&[&[C]], &[&[D]]);
    let carrier = Rule::new(
        vec![particle(&[A])],
        vec![Output::Particle(Particle::from(vec![
            Value::Atom(Atom(B)),
            nested(inner.clone()),
        ]))],
    );
    let scope = Rule::new(vec![particle(&[B])], Output::group(Vec::new(), vec![inner]));
    Program::from(vec![carrier, scope])
}

fn scope(output: &Output) -> &Scope {
    let Output::Scope(scope) = output else {
        panic!("expected a scope, found {output:?}");
    };
    scope
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
    assert!(matches!(
        &deleted.rule()[0].output()[0],
        Output::Particle(particle) if particle.flat().is_some()
    ));
    let doubled = transform(&program, 3, |rule| vec![rule.clone(), rule.clone()]);
    assert_eq!(scope(&doubled.rule()[1].output()[0]).rule().len(), 2);
    let emptied = transform(&program, 3, |_| Vec::new());
    assert_eq!(
        emptied.rule()[1].output(),
        [Output::Particle(Particle::default())]
    );
    let replaced = transform(&program, 0, |_| vec![rule(&[&[D]], &[])]);
    assert_eq!(walk(&replaced).len(), 3);
    assert!(replaced.rule().iter().any(|rule| rule.output().is_empty()));
    assert_eq!(transform(&program, 9, |_| Vec::new()), program);
}

#[test]
fn nesting() {
    let inner = rule(&[&[C]], &[&[D]]);
    let valued = rule(&[&[D]], &[&[B]]);
    let enclosed = Output::group(
        vec![Output::Particle(Particle::from(vec![nested(
            inner.clone(),
        )]))],
        vec![valued.clone()],
    );
    let output = Output::group(
        [particle(&[B]), particle(&[C])]
            .into_iter()
            .map(Output::Particle)
            .chain(enclosed)
            .collect(),
        vec![inner.clone()],
    );
    let root = Output::group(Vec::new(), vec![valued.clone()]);
    let [Output::Scope(root)] = &root[..] else {
        panic!("a group that lists a rule is a scope");
    };
    let program = Program::new(
        vec![Rule::new(vec![particle(&[A])], output)],
        vec![root.clone()],
    );
    let node = walk(&program);
    assert_eq!(
        node.iter()
            .map(|node| (node.rule.clone(), node.depth, node.place))
            .collect::<Vec<_>>(),
        [
            (program.rule()[0].clone(), 0, Place::Program),
            (inner.clone(), 1, Place::Body { parent: 0 }),
            (inner, 1, Place::Value { parent: 0 }),
            (valued, 1, Place::Body { parent: 0 }),
        ]
    );
    let deleted = transform(&program, 3, |_| Vec::new());
    let outer = scope(&deleted.rule()[0].output()[0]);
    assert!(outer.scope().is_empty());
    assert_eq!(outer.coherence().len(), 3);
    assert_eq!(deleted.scope(), program.scope());
}
