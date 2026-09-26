use crate::encoding::ATOM;
use crate::objective::{Evaluation, Setting, Size, TOLERANCE, evaluate, floor, memorization};
use crate::problem::Problem;
use crate::solution::{Budget, least, solve};
use crate::task::{Example, Task};
use code::atom::Atom;
use code::configuration::Configuration;
use code::observation::Observation;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use std::time::Duration;
use translation::lift;
use translation::vocabulary::Vocabulary;

const WIDE: Budget = Budget {
    size: 16,
    time: None,
};

const NARROW: Budget = Budget {
    size: 7,
    time: None,
};

fn parse(text: &str, vocabulary: &mut Vocabulary) -> (Program, Configuration) {
    lift::program(&frontend::lowering::parse(text).unwrap(), vocabulary).unwrap()
}

fn task(reference: Option<&str>) -> Task {
    let mut vocabulary = Vocabulary::default();
    let reference = reference.map(|text| parse(text, &mut vocabulary).0);
    let (_, input) = parse("A", &mut vocabulary);
    let (_, output) = parse("B", &mut vocabulary);
    Task {
        name: "rename".to_owned(),
        vocabulary,
        example: vec![Example {
            input,
            output: Observation::from(&output),
        }],
        holdout: Vec::new(),
        reference,
        goal: None,
    }
}

fn tested(pair: &[(&str, &str)]) -> Task {
    let mut vocabulary = Vocabulary::default();
    let example = pair
        .iter()
        .map(|(input, output)| Example {
            input: parse(input, &mut vocabulary).1,
            output: Observation::from(&parse(output, &mut vocabulary).1),
        })
        .collect();
    Task {
        name: "tested".to_owned(),
        vocabulary,
        example,
        holdout: Vec::new(),
        reference: None,
        goal: None,
    }
}

#[test]
fn improvement() {
    let setting = Setting::default();
    let task = task(Some("[A] (B), [C] ()"));
    let reference = task.reference.as_ref().unwrap();
    let known = evaluate(reference, &task.example, &task.vocabulary, &setting);
    assert!(known.correct);
    let solution = solve(&task, known.cost, &setting, WIDE).unwrap();
    assert!(solution.proven && solution.complete);
    assert!((solution.floor - 1.25).abs() < TOLERANCE);
    assert!((solution.cost - 1.6).abs() < TOLERANCE);
    assert!(solution.cost < known.cost - 0.2);
    assert_eq!(solution.size, Some(7));
    assert!(!solution.optimal.is_empty());
    for (program, evaluation) in &solution.optimal {
        assert_eq!(evaluation.size.total(), 7);
        let again = evaluate(program, &task.example, &task.vocabulary, &setting);
        assert!((again.cost - solution.cost).abs() < TOLERANCE);
    }
}

#[test]
fn blind() {
    let setting = Setting::default();
    let task = task(None);
    assert!((floor(&task.example, &task.vocabulary, &setting) - 1.25).abs() < TOLERANCE);
    let table = memorization(&task.example, &task.vocabulary, &setting);
    assert!((table - 1.6).abs() < TOLERANCE);
    let problem = Problem::new(task.clone(), &setting, ATOM).unwrap();
    assert!((problem.baseline - table).abs() < TOLERANCE);
    let solution = solve(&task, f64::INFINITY, &setting, WIDE).unwrap();
    assert!(solution.proven && solution.complete);
    assert!((solution.cost - 1.6).abs() < TOLERANCE);
    assert_eq!(solution.optimal.len(), 1);
    let tight = Budget {
        size: 6,
        time: None,
    };
    let solution = solve(&task, 1.6, &setting, tight).unwrap();
    assert!(solution.proven && !solution.complete);
    assert!(solution.cost.is_infinite());
}

#[test]
fn bounded() {
    let setting = Setting::default();
    let task = task(None);
    let small = Budget {
        size: 3,
        time: None,
    };
    let solution = solve(&task, f64::INFINITY, &setting, small).unwrap();
    assert!(!solution.proven);
    assert!(solution.cost.is_infinite());
    assert_eq!(solution.size, Some(3));
    assert!((solution.gap() - 1.45).abs() < TOLERANCE);
    assert!((least(&task, &setting) - 1.25).abs() < TOLERANCE);
    let expired = Budget {
        size: 16,
        time: Some(Duration::ZERO),
    };
    let solution = solve(&task, f64::INFINITY, &setting, expired).unwrap();
    assert!(!solution.proven);
    assert_eq!(solution.size, None);
    assert_eq!(solution.examined, 0);
}

#[test]
fn valued() {
    let setting = Setting::default();
    let task = tested(&[("A.Z.([A.A] B)", "Z.([A.A] B)")]);
    let (removal, _) = parse("[A] ()", &mut task.vocabulary.clone());
    let solution = solve(&task, f64::INFINITY, &setting, NARROW).unwrap();
    assert!(solution.proven && solution.complete);
    assert!((solution.cost - 1.5).abs() < TOLERANCE);
    assert_eq!(
        solution
            .optimal
            .iter()
            .map(|(program, _)| program)
            .collect::<Vec<_>>(),
        vec![&removal]
    );
}

fn particle(atom: &[Atom], limit: usize) -> Vec<Vec<Atom>> {
    let mut result = vec![Vec::new()];
    let mut frontier = vec![Vec::new()];
    for _ in 0..limit {
        frontier = frontier
            .iter()
            .flat_map(|particle: &Vec<Atom>| {
                atom.iter()
                    .filter(|&&next| particle.last().is_none_or(|&last| last <= next))
                    .map(|&next| {
                        let mut longer = particle.clone();
                        longer.push(next);
                        longer
                    })
            })
            .collect();
        result.extend(frontier.iter().cloned());
    }
    result
}

fn multiset(cost: &[usize], budget: usize, start: usize) -> Vec<Vec<usize>> {
    let mut result = vec![Vec::new()];
    for index in start..cost.len() {
        if cost[index] > budget {
            continue;
        }
        for mut rest in multiset(cost, budget - cost[index], index) {
            rest.insert(0, index);
            result.push(rest);
        }
    }
    result
}

fn exhaustive(task: &Task, size: usize, setting: &Setting) -> Vec<(Program, Evaluation)> {
    let atom = task.vocabulary.atom().collect::<Vec<_>>();
    let particle = particle(&atom, size.saturating_sub(2));
    let weight = particle
        .iter()
        .map(|particle| 1 + particle.len())
        .collect::<Vec<_>>();
    let price = |choice: &[usize]| choice.iter().map(|&index| weight[index]).sum::<usize>();
    let build = |choice: &[usize]| {
        choice
            .iter()
            .map(|&index| Particle::atom(&particle[index]))
            .collect::<Vec<_>>()
    };
    let side = multiset(&weight, size.saturating_sub(1), 0);
    let mut rule = Vec::new();
    let mut cost = Vec::new();
    for input in side.iter().filter(|input| !input.is_empty()) {
        for output in &side {
            let total = 1 + price(input) + price(output);
            if total <= size {
                rule.push(Rule::new(
                    build(input),
                    build(output).into_iter().map(Output::Particle).collect(),
                ));
                cost.push(total);
            }
        }
    }
    let mut best: Vec<(Program, Evaluation)> = Vec::new();
    for choice in multiset(&cost, size, 0) {
        let program = Program::from(
            choice
                .iter()
                .map(|&index| rule[index].clone())
                .collect::<Vec<_>>(),
        );
        if Size::new(&program).total() > size {
            continue;
        }
        let evaluation = evaluate(&program, &task.example, &task.vocabulary, setting);
        if !evaluation.correct {
            continue;
        }
        let least = best.first().map_or(f64::INFINITY, |(_, known)| known.cost);
        if evaluation.cost < least - TOLERANCE {
            best.clear();
        }
        if evaluation.cost <= least + TOLERANCE {
            best.push((program, evaluation));
        }
    }
    best.sort_by(|left, right| left.0.cmp(&right.0));
    best
}

#[test]
fn differential() {
    let setting = Setting::default();
    for task in [
        task(None),
        tested(&[("A", "B"), ("C", "C")]),
        tested(&[("A.X", "B.X"), ("A", "B")]),
        tested(&[("A.Z.([A.A] B)", "Z.([A.A] B)")]),
    ] {
        let solution = solve(&task, f64::INFINITY, &setting, NARROW).unwrap();
        assert!(solution.proven && solution.complete);
        let size = solution.size.unwrap();
        let expected = exhaustive(&task, size, &setting);
        let found = solution
            .optimal
            .iter()
            .map(|(program, _)| program)
            .collect::<Vec<_>>();
        assert_eq!(
            found,
            expected
                .iter()
                .map(|(program, _)| program)
                .collect::<Vec<_>>(),
            "{:?}",
            task.example
        );
        assert!((expected[0].1.cost - solution.cost).abs() < TOLERANCE);
    }
}
