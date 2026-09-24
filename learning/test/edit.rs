use crate::corpus::{gnome, sort};
use crate::edit::{Action, Bound, Local, Side, apply, legal, perturb};
use crate::objective::{Setting, evaluate};
use code::atom::Atom;
use code::program::Program;
use code::tree::{Place, walk};
use machine::limit::Limit;
use random::Generator;

#[test]
fn closure() {
    let task = sort(2, 3);
    let bound = Bound {
        depth: 2,
        ..Bound::default()
    };
    let mut generator = Generator::new(4);
    let mut program = task.reference.clone().unwrap();
    for _ in 0..200 {
        let action = legal(&program, task.vocabulary.len(), &bound);
        assert_eq!(action[0], Action::Stop);
        for &choice in &action {
            apply(&program, choice);
        }
        program = apply(&program, action[1 + generator.below(action.len() - 1)]);
        assert!(walk(&program).len() <= bound.rule + 1);
    }
}

#[test]
fn structure() {
    let empty = Program::default();
    let created = apply(&empty, Action::Create { atom: Atom(2) });
    assert_eq!(created.rule().len(), 1);
    assert_eq!(apply(&created, Action::Stop), created);
    let nested = apply(
        &created,
        Action::Nest {
            rule: 0,
            side: Side::Output,
            particle: 0,
            atom: Atom(1),
        },
    );
    let node = walk(&nested);
    assert_eq!(node.len(), 2);
    assert_eq!(node[1].place, Place::Value { parent: 0 });
    let enclosed = apply(
        &created,
        Action::Enclose {
            rule: 0,
            output: 0,
            atom: Atom(3),
        },
    );
    assert_eq!(walk(&enclosed)[1].place, Place::Body { parent: 0 });
    let unwrapped = apply(&enclosed, Action::Delete { rule: 1 });
    assert!(unwrapped.rule()[0].output()[0].body().is_none());
    let extended = apply(
        &created,
        Action::Insert {
            rule: 0,
            side: Side::Input,
            particle: 1,
            atom: Atom(0),
        },
    );
    assert_eq!(extended.rule()[0].input().len(), 2);
    assert_eq!(
        apply(
            &extended,
            Action::Close {
                rule: 0,
                side: Side::Input,
                particle: 0,
            }
        )
        .rule()[0]
            .input()
            .len(),
        1
    );
}

#[test]
fn perturbation() {
    let task = sort(3, 2);
    let mut generator = Generator::new(9);
    let changed = perturb(
        task.reference.as_ref().unwrap(),
        task.vocabulary.len(),
        &Bound::default(),
        3,
        &mut generator,
    );
    assert_ne!(Some(changed), task.reference);
}

#[test]
fn detach() {
    let task = gnome(2, 2);
    let gnome = task.vocabulary.find("Gnome").unwrap();
    let text = |program: &Program| {
        program
            .rule()
            .iter()
            .map(|rule| translation::text::rule(rule, &task.vocabulary))
            .collect::<Vec<_>>()
    };
    let swap = text(task.reference.as_ref().unwrap())
        .iter()
        .position(|rule| {
            rule == "[Gnome.Second, First.1, Second.0] (Gnome.First) (First.0) (Second.1)"
        })
        .unwrap();
    let action = legal(
        task.reference.as_ref().unwrap(),
        task.vocabulary.len(),
        &Bound::default(),
    );
    assert!(action.contains(&Action::Detach {
        rule: swap,
        atom: gnome,
    }));
    let start = text(task.reference.as_ref().unwrap())
        .iter()
        .position(|rule| rule == "[Gnome.First] (Gnome.Second)")
        .unwrap();
    assert!(!action.contains(&Action::Detach {
        rule: start,
        atom: gnome,
    }));
    let detached = apply(
        task.reference.as_ref().unwrap(),
        Action::Detach {
            rule: swap,
            atom: gnome,
        },
    );
    assert!(text(&detached).contains(&"[First.1, Second.0] (First.0) (Second.1)".to_owned()));
}

#[test]
fn ascent() {
    let task = gnome(3, 2);
    let base = Setting::default();
    let setting = Setting {
        limit: Limit {
            state: 48,
            event: 64,
            ..base.limit
        },
        budget: 256,
        sample: 0,
        ..base
    };
    let judge = |program: &Program| evaluate(program, &task.example, &task.vocabulary, &setting);
    let reference = judge(task.reference.as_ref().unwrap()).cost;
    let mut program = task.reference.clone().unwrap();
    let mut cost = reference;
    loop {
        let best = legal(&program, task.vocabulary.len(), &Bound::default())
            .into_iter()
            .map(|action| apply(&program, action))
            .map(|candidate| (judge(&candidate), candidate))
            .filter(|(evaluation, _)| evaluation.correct)
            .min_by(|left, right| left.0.cost.total_cmp(&right.0.cost));
        let Some((evaluation, candidate)) = best.filter(|(evaluation, _)| evaluation.cost < cost)
        else {
            break;
        };
        program = candidate;
        cost = evaluation.cost;
    }
    println!(
        "{reference} -> {cost}\n{}",
        translation::text::program(&program, &task.vocabulary)
    );
    assert!(cost < 0.5 * reference, "{reference} -> {cost}");
}

#[test]
fn analogy() {
    for (length, value) in [(4, 2), (5, 3)] {
        let task = gnome(length, value);
        let gnome = task.vocabulary.find("Gnome").unwrap();
        let swap = task
            .reference
            .as_ref()
            .unwrap()
            .rule()
            .iter()
            .position(|rule| {
                translation::text::rule(rule, &task.vocabulary)
                    == "[Gnome.Second, First.1, Second.0] (Gnome.First) (First.0) (Second.1)"
            })
            .unwrap();
        let action = legal(
            task.reference.as_ref().unwrap(),
            task.vocabulary.len(),
            &Bound::default(),
        );
        assert!(
            action
                .iter()
                .any(|action| matches!(action, Action::Analogy(Local::Detach { .. })))
        );
        let released = apply(
            task.reference.as_ref().unwrap(),
            Action::Analogy(Local::Detach {
                rule: swap,
                atom: gnome,
            }),
        );
        let exchange = released
            .rule()
            .iter()
            .filter(|rule| {
                rule.input().len() == 2
                    && translation::text::rule(rule, &task.vocabulary)
                        .split(['[', ']', ',', ' ', '(', ')', '.'])
                        .all(|word| word != "Gnome")
            })
            .count();
        assert_eq!(exchange, (length - 1) * value * (value - 1) / 2);
        let setting = Setting::default();
        let reference = evaluate(
            task.reference.as_ref().unwrap(),
            &task.example,
            &task.vocabulary,
            &setting,
        );
        let evaluation = evaluate(&released, &task.example, &task.vocabulary, &setting);
        assert!(evaluation.correct);
        assert!(evaluation.cost < reference.cost);
    }
}
