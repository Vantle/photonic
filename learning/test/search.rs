use crate::edit::{Action, apply};
use crate::objective::{Evaluation, Size};
use crate::search::{Environment, Position, Setting, Tree, sequence};
use code::atom::Atom;
use code::program::Program;
use random::Generator;
use std::sync::Arc;

#[test]
fn halving() {
    assert_eq!(sequence(4, 8), vec![0, 0, 0, 0, 1, 1, 2, 2]);
    assert_eq!(sequence(2, 5), vec![0, 0, 1, 1, 2]);
    assert_eq!(sequence(1, 3), vec![0, 1, 2]);
}

struct Toy;

fn position(potential: f64) -> Position {
    Position {
        program: Arc::new(Program::default()),
        evaluation: Some(exact()),
        potential,
    }
}

fn exact() -> Arc<Evaluation> {
    Arc::new(Evaluation {
        outcome: Vec::new(),
        correctness: 1.0,
        correct: true,
        verified: true,
        time: 0.0,
        work: 0.0,
        span: 0.0,
        size: Size::default(),
        cost: 0.0,
    })
}

impl Environment for Toy {
    fn transition(&mut self, current: &Position, action: Action) -> Position {
        let change = match action {
            Action::Create { atom: Atom(0) } => 1.0,
            Action::Create { atom: Atom(1) } => -1.0,
            _ => 0.0,
        };
        position(current.potential + change)
    }

    fn verify(&mut self, _: &Position, current: &Position) -> Position {
        current.clone()
    }

    fn legal(&mut self, _: &Position) -> Vec<Action> {
        vec![
            Action::Stop,
            Action::Create { atom: Atom(1) },
            Action::Create { atom: Atom(0) },
        ]
    }
}

#[test]
fn improvement() {
    let setting = Setting {
        simulation: 12,
        considered: 3,
        step: 2,
        ..Setting::default()
    };
    let mut environment = Toy;
    let mut generator = Generator::new(1);
    let root = position(0.0);
    let action = environment.legal(&root);
    let mut tree = Tree::new(root, 0, action, setting);
    tree.prepare(vec![0.0; 3], 0.0, &mut generator);
    while !tree.finished() {
        if let Some(leaf) = tree.simulate(&mut environment) {
            tree.expand(leaf, vec![0.0; 3], 0.0, 0.0);
        }
    }
    let chosen = tree.decide();
    assert_eq!(tree.action()[chosen], Action::Create { atom: Atom(0) });
    let policy = tree.policy();
    assert!(policy[2] > policy[0] && policy[0] > policy[1]);
    assert!(tree.value() > 0.0);
}

struct Blind {
    checked: usize,
    inferred: usize,
}

fn truth(program: &Program) -> f64 {
    program
        .rule()
        .iter()
        .map(|rule| {
            if rule.input()[0].value()[0] == code::value::Value::Atom(Atom(0)) {
                1.0
            } else {
                -1.0
            }
        })
        .sum()
}

impl Environment for Blind {
    fn transition(&mut self, current: &Position, action: Action) -> Position {
        if action == Action::Stop {
            return current.clone();
        }
        self.inferred += 1;
        Position {
            program: Arc::new(apply(&current.program, action)),
            evaluation: None,
            potential: f64::NAN,
        }
    }

    fn verify(&mut self, _: &Position, current: &Position) -> Position {
        self.checked += 1;
        Position {
            program: current.program.clone(),
            evaluation: Some(exact()),
            potential: truth(&current.program),
        }
    }

    fn legal(&mut self, _: &Position) -> Vec<Action> {
        vec![
            Action::Stop,
            Action::Create { atom: Atom(1) },
            Action::Create { atom: Atom(0) },
        ]
    }
}

#[test]
fn inference() {
    let setting = Setting {
        simulation: 16,
        considered: 3,
        step: 3,
        ..Setting::default()
    };
    let mut environment = Blind {
        checked: 0,
        inferred: 0,
    };
    let mut generator = Generator::new(2);
    let root = position(0.0);
    let action = environment.legal(&root);
    let mut tree = Tree::new(root, 0, action, setting);
    tree.prepare(vec![0.0; 3], 0.0, &mut generator);
    while !tree.finished() {
        if let Some(leaf) = tree.simulate(&mut environment) {
            tree.expand(leaf, vec![0.0; 3], 0.0, 0.0);
        }
    }
    assert_eq!(
        tree.action()[tree.decide()],
        Action::Create { atom: Atom(0) }
    );
    assert!(environment.checked > 0);
    assert!(environment.checked < environment.inferred);
}
