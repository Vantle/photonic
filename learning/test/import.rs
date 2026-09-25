use crate::edit::{Action, apply};
use crate::import::{Failure, Source, define, import};
use crate::objective::{Setting, evaluate};

fn source(text: &str) -> Source {
    Source {
        origin: "memory".to_owned(),
        text: text.to_owned(),
    }
}

const PROGRAM: &str = "
[Invoke] (Function, [Return] ()),
[Function.Boolean.Not.True] Return.False,
[Function.Boolean.Not.False] Return.True
";

#[test]
fn scoped() {
    let setting = Setting::default();
    let task = import(
        "negation",
        &[source(PROGRAM)],
        &[
            source("Invoke.Boolean.Not.True"),
            source("Invoke.Boolean.Not.False"),
        ],
        &setting,
    )
    .unwrap();
    assert_eq!(task.example.len(), 2);
    assert!(!task.reference.as_ref().unwrap().flat());
    let expected = ["False", "True"];
    for (example, name) in task.example.iter().zip(expected) {
        let configuration = example.output.configuration();
        assert_eq!(configuration.coherence().len(), 1);
        let atom = task.vocabulary.find(name).unwrap();
        assert_eq!(
            configuration.coherence()[0].flat(),
            Some(vec![atom]),
            "{name}"
        );
    }
    let evaluation = evaluate(
        task.reference.as_ref().unwrap(),
        &task.example,
        &task.vocabulary,
        &setting,
    );
    assert!(evaluation.correct);
    assert!(evaluation.verified);
    let broken = apply(task.reference.as_ref().unwrap(), Action::Delete { rule: 0 });
    assert!(!evaluate(&broken, &task.example, &task.vocabulary, &setting).correct);
}

#[test]
fn initial() {
    let task = import(
        "own",
        &[source("A.X, [A] (B, C), [B, C] D")],
        &[],
        &Setting::default(),
    )
    .unwrap();
    assert_eq!(task.example.len(), 1);
}

#[test]
fn rejection() {
    let setting = Setting::default();
    assert!(matches!(
        import("empty", &[source("[A] B")], &[], &setting),
        Err(Failure::Empty)
    ));
    assert!(matches!(
        import("rules", &[source("[A] B")], &[source("A, [C] D")], &setting),
        Err(Failure::Input { .. })
    ));
    assert!(matches!(
        import(
            "choice",
            &[source("[A] B, [A] C")],
            &[source("A")],
            &setting
        ),
        Err(Failure::Behavior { .. })
    ));
}

#[test]
fn defined() {
    let task = define(
        "rename",
        &[(source("A.X"), source("B.X")), (source("A"), source("B"))],
    )
    .unwrap();
    assert!(task.reference.is_none());
    assert_eq!(task.example.len(), 2);
    assert!(task.vocabulary.find("B").is_some());
    assert!(matches!(define("empty", &[]), Err(Failure::Untested)));
    assert!(matches!(
        define("ruled", &[(source("[A] B"), source("B"))]),
        Err(Failure::Input { .. })
    ));
}
