use crate::corpus::{addition, boolean, maximum, sort};
use crate::objective::{Outcome, Setting, Size, evaluate, potential};
use crate::task::Example;
use code::atom::Atom;
use code::observation::Observation;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::value::Value;
use translation::lift;
use translation::vocabulary::Vocabulary;

fn inert(program: &Program, atom: Atom) -> Program {
    let value = Rule::new(
        vec![Particle::atom(&[atom])],
        vec![Output::plain(Particle::default())],
    );
    let carrier = Rule::new(
        vec![Particle::atom(&[atom])],
        vec![Output::plain(Particle::from(vec![Value::Rule(Box::new(
            value,
        ))]))],
    );
    let mut rule = program.rule().to_vec();
    rule.push(carrier);
    Program::from(rule)
}

#[test]
fn agreement() {
    let setting = Setting::default();
    let task = boolean().into_iter().nth(1).unwrap();
    for task in [sort(2, 3), sort(3, 2), maximum(3, 3), addition(2), task] {
        let task = task.conceal(1);
        let silent = code::atom::Atom((task.vocabulary.len() - 1) as u16);
        for program in [task.reference.clone().unwrap(), Program::default()] {
            let flat = evaluate(&program, &task.example, &task.vocabulary, &setting);
            let nested = evaluate(
                &inert(&program, silent),
                &task.example,
                &task.vocabulary,
                &setting,
            );
            assert_eq!(flat.outcome, nested.outcome, "{}", task.name);
        }
    }
}

#[test]
fn ordering() {
    let setting = Setting::default();
    let task = sort(2, 3);
    let reference = evaluate(
        task.reference.as_ref().unwrap(),
        &task.example,
        &task.vocabulary,
        &setting,
    );
    let empty = evaluate(
        &Program::default(),
        &task.example,
        &task.vocabulary,
        &setting,
    );
    assert!(reference.correct && reference.verified);
    assert!(!empty.correct);
    assert!(empty.correctness > 0.0 && empty.correctness < 1.0);
    let baseline = reference.cost;
    assert!(potential(&reference, baseline) > potential(&empty, baseline));
    assert!((potential(&reference, baseline) - 2.0).abs() < 1e-9);
    assert!(reference.time > 0.0);
    assert_eq!(Size::new(&Program::default()).total(), 0);
}

fn single(program: &str, input: &str, output: &str) -> (Program, Example, Vocabulary) {
    let mut vocabulary = Vocabulary::default();
    let (program, _) = lift::program(
        &photonic::lowering::parse(program).unwrap(),
        &mut vocabulary,
    )
    .unwrap();
    let (_, input) =
        lift::program(&photonic::lowering::parse(input).unwrap(), &mut vocabulary).unwrap();
    let (_, output) =
        lift::program(&photonic::lowering::parse(output).unwrap(), &mut vocabulary).unwrap();
    (
        program,
        Example {
            input,
            output: Observation::from(&output),
        },
        vocabulary,
    )
}

#[test]
fn failure() {
    let setting = Setting::default();
    let (program, example, vocabulary) = single("[A] B, [A] C", "A", "B");
    let result = evaluate(&program, &[example], &vocabulary, &setting);
    assert!(matches!(result.outcome[0], Outcome::Choice { .. }));
    let (program, example, vocabulary) = single("[A] A.A", "A", "B");
    let result = evaluate(&program, &[example], &vocabulary, &setting);
    assert_eq!(result.outcome[0], Outcome::Divergent);
    let (program, example, vocabulary) = single("[A] (B, C)", "A.X", "B.X, C.X");
    let result = evaluate(&program, &[example], &vocabulary, &setting);
    assert!(matches!(result.outcome[0], Outcome::Different { .. }));
}
