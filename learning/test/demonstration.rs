use crate::corpus::{addition, boolean};
use crate::demonstration::demonstrate;
use crate::edit::{Action, Bound, apply};
use crate::objective::Setting;
use code::program::Program;
use random::Generator;
use translation::lift;

fn program(text: &str, task: &crate::task::Task) -> Program {
    let mut vocabulary = task.vocabulary.clone();
    let (program, _) =
        lift::program(&photonic::lowering::parse(text).unwrap(), &mut vocabulary).unwrap();
    assert_eq!(vocabulary.len(), task.vocabulary.len());
    program
}

#[test]
fn path() {
    let setting = Setting::default();
    let task = addition(2);
    for text in ["[Left, Right] ()", "[Left, Right] (Sum), [Sum] ()"] {
        let target = program(text, &task);
        let demonstration = demonstrate(&task, &target, &Bound::default(), &setting).unwrap();
        let mut current = Program::default();
        for (index, step) in demonstration.step.iter().enumerate() {
            assert_eq!(step.program, current, "{text} step {index}");
            let action = step.action[step.chosen];
            if index + 1 == demonstration.step.len() {
                assert_eq!(action, Action::Stop);
            } else {
                current = apply(&current, action);
            }
        }
        assert_eq!(current, target);
        assert!(demonstration.goal.correct);
    }
}

#[test]
fn sample() {
    let setting = Setting::default();
    let task = boolean().remove(1);
    let target = program("[And.True] (), [And.False.False] (False)", &task);
    let demonstration = demonstrate(&task, &target, &Bound::default(), &setting).unwrap();
    let mut generator = Generator::new(4);
    let sample = demonstration.sample(&task, 2.45, &mut generator);
    assert_eq!(sample.len(), 2 * demonstration.step.len() - 1);
    let policy = sample
        .iter()
        .filter(|sample| !sample.policy.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(policy.len(), demonstration.step.len());
    for sample in policy {
        assert_eq!(
            sample.policy.iter().filter(|value| **value == 1.0).count(),
            1
        );
        assert!(sample.value.is_some());
    }
    assert!(
        sample
            .iter()
            .filter(|sample| sample.policy.is_empty())
            .all(|sample| sample.judge.is_some())
    );
}
