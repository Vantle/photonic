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
        vec![Output::Particle(Particle::from(vec![nested(
            inner.clone(),
        )]))],
    );
    let scoped = Rule::new(vec![particle(&[B])], Output::group(Vec::new(), vec![inner]));
    assert!(!Program::from(vec![valued]).flat());
    assert!(!Program::from(vec![scoped.clone()]).flat());
    assert!(Program::from(vec![rule(&[&[A]], &[&[B]])]).flat());
    let Output::Scope(scope) = &scoped.output()[0] else {
        panic!("a group that lists a rule is a scope");
    };
    assert_eq!(scope.coherence(), [Particle::default()]);
    assert!(!Program::new(Vec::new(), vec![scope.clone()]).flat());
}

#[test]
fn group() {
    let inner = rule(&[&[C]], &[&[D]]);
    let member = vec![
        Output::Particle(particle(&[A])),
        Output::Particle(particle(&[B])),
    ];
    assert_eq!(Output::group(member.clone(), Vec::new()), member);
    let nested = Output::group(vec![Output::Particle(particle(&[C]))], vec![inner.clone()]);
    let outer = Output::group(
        member.into_iter().chain(nested.clone()).collect(),
        vec![inner.clone()],
    );
    let [Output::Scope(outer)] = &outer[..] else {
        panic!("a group that lists a rule is a scope");
    };
    assert_eq!(outer.coherence(), [particle(&[A]), particle(&[B])]);
    assert_eq!(outer.rule(), std::slice::from_ref(&inner));
    let [Output::Scope(nested)] = &nested[..] else {
        panic!("a group that lists a rule is a scope");
    };
    assert_eq!(outer.scope(), std::slice::from_ref(nested));
    let [Output::Scope(bare)] =
        &Output::group(vec![Output::Scope(nested.clone())], vec![inner])[..]
    else {
        panic!("a group that lists a rule is a scope");
    };
    assert!(bare.coherence().is_empty());
    let text = serde_json::to_string(&Output::Scope(outer.clone())).unwrap();
    assert_eq!(
        serde_json::from_str::<Output>(&text).unwrap(),
        Output::Scope(outer.clone())
    );
    assert!(serde_json::from_str::<Output>(r#"{"Scope": {"coherence": [], "rule": []}}"#).is_err());
}
