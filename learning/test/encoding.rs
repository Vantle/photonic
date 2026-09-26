use crate::corpus::{boolean, sort};
use crate::edit::{Action, Bound, Side, apply, legal};
use crate::encoding::{Permutation, Shape, encode};
use crate::objective::{Setting, evaluate};
use crate::task::{Goal, Task};
use code::atom::Atom;
use network::input::Pointer;
use network::model::Model;
use random::Generator;

#[test]
fn pointer() {
    let task = sort(2, 3).conceal(4).unwrap();
    let shape = Shape {
        width: 16,
        depth: 1,
        head: 2,
        hidden: 32,
        key: 8,
    };
    let architecture = shape.architecture();
    let mut generator = Generator::new(5);
    let model = Model::new(architecture.clone(), &mut generator);
    let bound = Bound {
        depth: 2,
        ..Bound::default()
    };
    let program = apply(
        &apply(
            task.reference.as_ref().unwrap(),
            Action::Nest {
                rule: 0,
                side: Side::Output,
                particle: 0,
                atom: Atom(1),
            },
        ),
        Action::Enclose {
            rule: 1,
            output: 0,
            atom: Atom(2),
        },
    );
    let evaluation = evaluate(
        &program,
        &task.example,
        &task.vocabulary,
        &Setting::default(),
    );
    let action = legal(&program, task.vocabulary.len(), &bound);
    let permutation = Permutation::new(&mut generator, task.example.len());
    let input = encode(&task, &program, &evaluation, &action, &permutation);
    let field = architecture.field.len();
    let length = input.length(field);
    assert_eq!(input.pointer.len(), action.len());
    for (index, value) in input.feature.iter().enumerate() {
        assert!(usize::from(*value) < architecture.field[index % field]);
    }
    for pointer in &input.pointer {
        match *pointer {
            Pointer::Unary { head, token } => {
                assert!(usize::from(head) < architecture.unary);
                assert!((token as usize) < length);
            }
            Pointer::Binary { head, left, right } => {
                assert!(usize::from(head) < architecture.binary);
                assert!((left as usize) < length && (right as usize) < length);
            }
        }
    }
    let output = model.infer(&[&input]);
    assert_eq!(output[0].logit.len(), action.len());
}

#[test]
fn goal() {
    let task = boolean().remove(0);
    let program = task.reference.clone().unwrap();
    let evaluation = evaluate(
        &program,
        &task.example,
        &task.vocabulary,
        &Setting::default(),
    );
    let permutation = Permutation::new(&mut Generator::new(1), task.example.len());
    let field = Shape::default().architecture().field.len();
    let plain = encode(&task, &program, &evaluation, &[], &permutation);
    let aimed = Task {
        goal: Some(Goal::new(64.0, 0.2).unwrap()),
        ..task
    };
    let aimed = encode(&aimed, &program, &evaluation, &[], &permutation);
    assert_ne!(plain.feature[..field], aimed.feature[..field]);
    assert_eq!(plain.feature[field..], aimed.feature[field..]);
}
