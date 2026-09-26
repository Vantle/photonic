use crate::corpus::{addition, boolean, maximum, sort};
use crate::objective::{Outcome, Setting, Size, TOLERANCE, evaluate, potential};
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
        vec![Output::Particle(Particle::default())],
    );
    let carrier = Rule::new(
        vec![Particle::atom(&[atom])],
        vec![Output::Particle(Particle::from(vec![Value::Rule(
            Box::new(value),
        )]))],
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
        let task = task.conceal(1).unwrap();
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
    assert!((potential(&reference, baseline) - 2.0).abs() < TOLERANCE);
    assert!(reference.time > 0.0);
    assert_eq!(Size::new(&Program::default()).total(), 0);
}

fn single(program: &str, input: &str, output: &str) -> (Program, Example, Vocabulary) {
    let mut vocabulary = Vocabulary::default();
    let (program, _) = lift::program(
        &frontend::lowering::parse(program).unwrap(),
        &mut vocabulary,
    )
    .unwrap();
    let (_, input) =
        lift::program(&frontend::lowering::parse(input).unwrap(), &mut vocabulary).unwrap();
    let (_, output) =
        lift::program(&frontend::lowering::parse(output).unwrap(), &mut vocabulary).unwrap();
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

#[test]
fn exhaustion() {
    let task = addition(2);
    let setting = Setting {
        budget: 1,
        ..Setting::default()
    };
    let result = evaluate(
        task.reference.as_ref().unwrap(),
        &task.example,
        &task.vocabulary,
        &setting,
    );
    assert!(result.correct && !result.verified);
}

#[test]
fn spent() {
    let (program, example, vocabulary) = single("[A] B, [A] C", "A", "B");
    let setting = Setting {
        budget: 1,
        sample: 16,
        ..Setting::default()
    };
    let result = evaluate(&program, &[example.clone(), example], &vocabulary, &setting);
    assert!(matches!(result.outcome[0], Outcome::Choice { .. }));
    assert!(matches!(
        result.outcome[1],
        Outcome::Exact {
            verified: false,
            ..
        }
    ));
}

#[test]
fn schedule() {
    let bit = 8;
    let program = (0..bit)
        .map(|index| {
            format!(
                "[Carry.B{index}, B{index}.Zero] (B{index}.One, Carry.B0),\n[Carry.B{index}, B{index}.One] (B{index}.Zero, Carry.B{}),\n",
                index + 1
            )
        })
        .chain(std::iter::once(format!("[Carry.B{bit}]")))
        .collect::<String>();
    let input = std::iter::once("Carry.B0".to_owned())
        .chain((0..bit).map(|index| {
            let value = if index + 1 < bit { "One" } else { "Zero" };
            format!("B{index}.{value}")
        }))
        .collect::<Vec<_>>()
        .join(", ");
    let output = (0..bit)
        .map(|index| format!("B{index}.Zero"))
        .collect::<Vec<_>>()
        .join(", ");
    let (program, example, vocabulary) = single(&program, &input, &output);
    let result = evaluate(&program, &[example], &vocabulary, &Setting::default());
    assert!(result.correct && result.verified);
    assert!(result.span > 256.0);
}
