use super::support::{BUG, FIX, ORIGINAL, engine, explore, plain, survey};
use crate::cause::Role;
use crate::claim::{self, Answer, Claim, Kind};
use crate::configuration::{Opener, Value};
use crate::lineage;
use crate::recording::{Engine, Mode, Order};
use crate::render;

fn claim(kind: Kind, pattern: &str) -> Claim {
    Claim {
        kind,
        pattern: pattern.to_owned(),
        exact: false,
        preserve: false,
    }
}

#[test]
fn size() {
    let bug = explore(BUG);
    assert_eq!(bug.order, Order::Shape);
    assert_eq!(bug.shape, Some(0x111d_39d9_6ed3_2425));
    assert!(bug.closed);
    assert_eq!(bug.configuration.len(), 18);
    assert_eq!(bug.event.len(), 25);
    assert_eq!(
        (0..bug.event.len())
            .filter(|&index| bug.inferred(index))
            .count(),
        6
    );
    assert_eq!(bug.work, 349);
    let original = explore(ORIGINAL);
    assert_eq!(
        (
            original.configuration.len(),
            original.event.len(),
            original.work
        ),
        (14, 17, 240)
    );
    let fix = explore(FIX);
    assert_eq!(
        (fix.configuration.len(), fix.event.len(), fix.work),
        (14, 18, 246)
    );
}

#[test]
fn handle() {
    let bug = explore(BUG);
    assert_eq!(render::configuration(&bug, 6), "False.True.Extra");
    assert_eq!(render::configuration(&bug, 13), "False.Extra");
    assert_eq!(render::configuration(&bug, 5), "in f1: False.True.Extra");
    assert_eq!(bug.rule[2].text, "[False] False");
    assert_eq!(bug.rule[2].scope, Some(Opener::Rule(1)));
    assert_eq!(bug.event[11].rule, 2);
    assert_eq!((bug.event[11].source, bug.event[11].target), (5, 6));
    assert_eq!(bug.path(6), Some(vec![4, 11]));
    assert_eq!(bug.event[4].deduction, vec![2, 8]);
}

#[test]
fn evaluation() {
    let bug = explore(BUG);
    let reach = claim::evaluate(&claim(Kind::Reach, "False.Extra"), &bug).unwrap();
    assert_eq!(reach.answer, Answer::Holds);
    assert_eq!(reach.witness.as_deref(), Some("s0"));
    assert!(reach.path.is_empty());
    let exact = claim::evaluate(
        &Claim {
            exact: true,
            preserve: true,
            ..claim(Kind::Reach, "False.Extra")
        },
        &bug,
    )
    .unwrap();
    assert_eq!(exact.answer, Answer::Holds);
    assert_eq!(exact.witness.as_deref(), Some("s13"));
    assert_eq!(exact.path, vec!["e0", "e7", "e19"]);
    let absent = claim::evaluate(&claim(Kind::Reach, "Nothing"), &bug).unwrap();
    assert_eq!(absent.answer, Answer::Fails);
    let avoid = claim::evaluate(&claim(Kind::Avoid, "False.True.Extra"), &bug).unwrap();
    assert_eq!(avoid.answer, Answer::Fails);
    assert_eq!(avoid.witness.as_deref(), Some("s0"));
    let always = claim::evaluate(&claim(Kind::Always, "Extra"), &bug).unwrap();
    assert_eq!(always.answer, Answer::Holds);
    let outcome = claim::evaluate(&claim(Kind::Outcome, "Boolean"), &bug).unwrap();
    assert_eq!(outcome.answer, Answer::Fails);
    let inevitable = claim::evaluate(&claim(Kind::Inevitable, "Extra"), &bug).unwrap();
    assert_eq!(inevitable.answer, Answer::Holds);
    let stuck = claim::evaluate(&claim(Kind::Inevitable, "Boolean.Boolean"), &bug).unwrap();
    assert_eq!(stuck.answer, Answer::Fails);
}

#[test]
fn lineage() {
    let bug = explore(BUG);
    let line = lineage::lineage(&bug, 6, 1).unwrap();
    let role = line.iter().map(|line| line.role).collect::<Vec<_>>();
    assert_eq!(role, vec![Role::Remainder, Role::Witness, Role::Initial]);
    assert_eq!(line[0].event, Some(11));
    assert_eq!(line[1].event, Some(4));
    assert_eq!((line[2].configuration, line[2].occurrence), (0, 2));
    let original = explore(ORIGINAL);
    let produced = lineage::lineage(&original, 6, 0).unwrap();
    assert_eq!(produced.len(), 1);
    assert_eq!(produced[0].role, Role::Produced);
    assert_eq!(produced[0].event, Some(8));
    assert_eq!(produced[0].source.len(), 3);
    let held = lineage::lineage(&original, 4, 0).unwrap();
    let role = held.iter().map(|line| line.role).collect::<Vec<_>>();
    assert_eq!(role, vec![Role::Held, Role::Initial]);
}

#[test]
fn invariance() {
    let bug = explore(BUG);
    let renamed = explore(&BUG.replace("Boolean", "Bit").replace("Extra", "Rest"));
    assert_ne!(bug.key, renamed.key);
    assert_eq!(bug.shape, renamed.shape);
    assert_eq!(bug.event.len(), renamed.event.len());
    assert_eq!(render::configuration(&renamed, 6), "False.True.Rest");
    let reversed = explore(&BUG.replace("Boolean", "Zeta").replace("Extra", "Alpha"));
    assert_eq!(bug.shape, reversed.shape);
    assert_eq!(render::configuration(&reversed, 6), "False.True.Alpha");
    assert_eq!(render::configuration(&reversed, 13), "False.Alpha");
    let reordered = explore(
        "[And.Boolean.Boolean] (\n    [False] False,\n    [True.True] True,\n),\n[False] Boolean,\n[True] Boolean,\nAnd.True.False.Extra\n",
    );
    assert_eq!(bug.key, reordered.key);
    assert_eq!(render::configuration(&reordered, 6), "False.True.Extra");
}

#[test]
fn symmetry() {
    for order in [
        [
            "Light, [Light] Red, [Light] Green, [Light] Blue",
            "Light, [Light] Blue, [Light] Green, [Light] Red",
            "[Light] Green, Light, [Light] Red, [Light] Blue",
        ],
        [
            "A.B, [A] C, [B] D",
            "[B] D, [A] C, B.A",
            "[A] C, B.A, [B] D",
        ],
    ] {
        let first = explore(order[0]);
        for source in &order[1..] {
            let other = explore(source);
            assert_eq!(first.key, other.key, "{source}");
            for (index, rule) in first.rule.iter().enumerate() {
                assert_eq!(rule.text, other.rule[index].text, "{source}");
            }
            for index in 0..first.configuration.len() {
                assert_eq!(
                    render::configuration(&first, index),
                    render::configuration(&other, index),
                    "{source}"
                );
            }
        }
    }
}

#[test]
fn opener() {
    let exploration = explore("A, B, [A] ([X] Y), [B] ([P] Q)");
    let mut seen = 0;
    for frame in exploration
        .configuration
        .iter()
        .flat_map(|configuration| configuration.frame.iter().skip(1))
    {
        let Some(Opener::Rule(opener)) = frame.opener else {
            panic!("a rule opens every scope of this program");
        };
        let body = exploration.rule[opener]
            .definition
            .output
            .iter()
            .flat_map(|output| match output {
                frontend::source::Output::Scope(program) => program.rule.as_slice(),
                frontend::source::Output::Particle(_) => &[],
            })
            .map(frontend::text::definition)
            .collect::<Vec<_>>();
        for occurrence in &frame.rule {
            let Value::Rule(rule) = occurrence.value else {
                continue;
            };
            assert!(
                body.contains(&exploration.rule[rule].text),
                "{} is not in the scope {} opens",
                exploration.rule[rule].text,
                exploration.rule[opener].text
            );
            assert_eq!(exploration.rule[rule].scope, Some(Opener::Rule(opener)));
            seen += 1;
        }
    }
    assert!(seen >= 2);
}

#[test]
fn inevitable() {
    let exploration = explore("A, [A] M, [A] N, [M] E, [N] P, [P] E");
    let verdict = claim::evaluate(&claim(Kind::Inevitable, "M"), &exploration).unwrap();
    assert_eq!(verdict.answer, Answer::Fails);
    assert!(!verdict.path.is_empty());
    let mut cursor = 0;
    for handle in &verdict.path {
        let event = &exploration.event[handle[1..].parse::<usize>().unwrap()];
        assert_eq!(event.source, cursor);
        cursor = event.target;
        assert_ne!(render::configuration(&exploration, cursor), "M");
    }
    let witness = verdict.witness.expect("a witness");
    assert_eq!(witness, format!("s{cursor}"));
    assert_eq!(render::configuration(&exploration, cursor), "E");
}

// Both engines close with the same configurations and events and answer every claim alike; only
// the order of their handles may differ.
#[test]
fn laser() {
    let pattern = [
        "False.Extra",
        "True.Extra",
        "Boolean",
        "Boolean.Boolean",
        "Extra",
        "Nothing",
        "B",
        "C",
    ];
    for source in [
        BUG,
        ORIGINAL,
        FIX,
        "A, [A] M, [A] N, [M] E, [N] P, [P] E",
        "A, B, [A] ([X] Y), [B] ([P] Q)",
        "A.B, [A] C, [B] D",
        &photonic::family::dial(3),
        &photonic::family::diner(2),
    ] {
        let interpreter = explore(source);
        let compiled = engine(source, Engine::Laser);
        assert_eq!(compiled.engine, Engine::Laser, "{source}");
        assert_ne!(interpreter.key, compiled.key, "{source}");
        assert!(interpreter.closed && compiled.closed, "{source}");
        let inferred = |exploration: &crate::exploration::Exploration| {
            (0..exploration.event.len())
                .filter(|&index| exploration.inferred(index))
                .count()
        };
        assert_eq!(
            (
                interpreter.configuration.len(),
                interpreter.event.len(),
                inferred(&interpreter)
            ),
            (
                compiled.configuration.len(),
                compiled.event.len(),
                inferred(&compiled)
            ),
            "{source}"
        );
        for kind in [
            Kind::Reach,
            Kind::Avoid,
            Kind::Always,
            Kind::Inevitable,
            Kind::Outcome,
        ] {
            for pattern in pattern {
                let answer = |exploration| {
                    claim::evaluate(&claim(kind, pattern), exploration)
                        .unwrap()
                        .answer
                };
                assert_eq!(
                    answer(&interpreter),
                    answer(&compiled),
                    "{source} {kind:?} {pattern}"
                );
            }
        }
        let target = frontend::lowering::parse(pattern[0]).unwrap();
        assert_eq!(
            interpreter
                .verdict(&target, true)
                .map(|verdict| (verdict.outcome, verdict.witness)),
            compiled
                .verdict(&target, true)
                .map(|verdict| (verdict.outcome, verdict.witness)),
            "{source}"
        );
        assert_eq!(
            interpreter.configuration, compiled.configuration,
            "{source}"
        );
        let shared = |exploration: &crate::exploration::Exploration| {
            exploration
                .event
                .iter()
                .map(|event| crate::exploration::Event {
                    deduction: Vec::new(),
                    ..event.clone()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(shared(&interpreter), shared(&compiled), "{source}");
        for index in 0..interpreter.configuration.len() {
            assert_eq!(interpreter.path(index), compiled.path(index), "{source}");
        }
        for (index, configuration) in compiled.configuration.iter().enumerate() {
            if compiled.path(index).is_none() {
                continue;
            }
            for occurrence in configuration
                .coherence
                .iter()
                .flat_map(|coherence| &coherence.occurrence)
            {
                let line = lineage::lineage(&compiled, index, occurrence.id).unwrap();
                let last = line.last().expect("a lineage has a line").role;
                assert!(
                    matches!(last, Role::Initial | Role::Produced),
                    "{source} s{index}.o{}",
                    occurrence.id
                );
            }
        }
    }
}

// Inference lets a rule consume what a configuration will become, so an inferred event can strand a
// run that every schedule of plain events would finish; plain mode answers for those schedules.
#[test]
fn schedule() {
    let source = "Claim, [Claim] P.Work, [Work] Done, [P] X";
    let full = explore(source);
    let every = plain(source);
    assert_eq!(every.mode, Mode::Plain);
    assert_eq!(every.engine, Engine::Laser);
    assert!(full.closed && every.closed);
    assert!(every.configuration.len() < full.configuration.len());
    assert_eq!(
        (0..every.event.len())
            .filter(|&index| every.inferred(index))
            .count(),
        0
    );
    for (kind, exhaustive, schedule) in [
        (Kind::Reach, Answer::Holds, Answer::Holds),
        (Kind::Inevitable, Answer::Fails, Answer::Holds),
        (Kind::Outcome, Answer::Fails, Answer::Holds),
    ] {
        let answer = |exploration| {
            claim::evaluate(&claim(kind, "Done"), exploration)
                .unwrap()
                .answer
        };
        assert_eq!(answer(&full), exhaustive, "{kind:?}");
        assert_eq!(answer(&every), schedule, "{kind:?}");
    }
    assert_ne!(full.key, every.key);
}

fn exact(kind: Kind, pattern: &str) -> Claim {
    Claim {
        exact: true,
        preserve: true,
        ..claim(kind, pattern)
    }
}

// An exact target is one whole configuration, so every claim can ask for it: which configuration a
// claim looks for changes, and how its kind decides does not.
#[test]
fn target() {
    let exploration = explore("A, [A] B");
    for (kind, pattern, expected) in [
        (Kind::Reach, "A", Answer::Holds),
        (Kind::Avoid, "B", Answer::Fails),
        (Kind::Always, "B", Answer::Fails),
        (Kind::Inevitable, "B", Answer::Holds),
        (Kind::Outcome, "B", Answer::Holds),
        (Kind::End, "B", Answer::Holds),
        (Kind::End, "A", Answer::Fails),
    ] {
        let verdict = claim::evaluate(&exact(kind, pattern), &exploration).unwrap();
        assert_eq!(verdict.answer, expected, "{kind:?} {pattern}");
    }
    let always = claim::evaluate(&exact(Kind::Always, "B"), &exploration).unwrap();
    assert_eq!(always.witness.as_deref(), Some("s0"));
    assert!(always.reason.contains("is not the target"));
}

// Every run ends, and at a match: a cycle any run reaches fails the claim at once, an end without a
// match fails it once the exploration closes, and outcome, which speaks only of ends, holds where
// no run ends at all.
#[test]
fn end() {
    let task = plain("Task.T1.Pending, Task.T2.Pending, [Pending] Running, [Running] Done");
    assert_eq!(task.endless(), Some(false));
    let done = claim::evaluate(&claim(Kind::End, "Done"), &task).unwrap();
    assert_eq!(done.answer, Answer::Holds);
    let whole = claim::evaluate(&exact(Kind::End, "Task.T1.Done, Task.T2.Done"), &task).unwrap();
    assert_eq!(whole.answer, Answer::Holds);
    let stray = claim::evaluate(&exact(Kind::End, "Task.T1.Done, Task.T2.Running"), &task).unwrap();
    assert_eq!(stray.answer, Answer::Fails);
    assert!(stray.witness.is_some());
    let dial = plain(&photonic::family::dial(2));
    assert_eq!(dial.endless(), Some(true));
    let endless = claim::evaluate(&claim(Kind::End, "Zero"), &dial).unwrap();
    assert_eq!(endless.answer, Answer::Fails);
    assert!(endless.reason.contains("forever"), "{}", endless.reason);
    assert!(!endless.path.is_empty());
    let outcome = claim::evaluate(&claim(Kind::Outcome, "Zero"), &dial).unwrap();
    assert_eq!(outcome.answer, Answer::Holds);
}

// Metal keeps only counts, ends and cycles, and those agree with laser's recording of the same plain
// schedules, as do its answers to every end and outcome claim; it refuses the claims that need every
// configuration.
#[test]
fn metal() {
    let pattern = ["Done", "Running", "Zero", "Nothing", "B", "C", "X"];
    for source in [
        "Task.T1.Pending, Task.T2.Pending, [Pending] Running, [Running] Done",
        "A, [A] B, [A] C",
        "Claim, [Claim] P.Work, [Work] Done, [P] X",
        "A.B, A.C, [A, B] X, [A, C] Y",
        &photonic::family::dial(3),
        &photonic::family::diner(2),
    ] {
        let recorded = plain(source);
        let survey = survey(source);
        assert_eq!(survey.closed, recorded.closed, "{source}");
        assert_eq!(
            survey.configuration,
            recorded.configuration.len(),
            "{source}"
        );
        assert_eq!(survey.event, recorded.event.len() as u64, "{source}");
        assert_eq!(survey.endless, recorded.endless(), "{source}");
        let mut end = recorded
            .leaf()
            .map(|index| render::configuration(&recorded, index))
            .collect::<Vec<_>>();
        let mut found = survey
            .end
            .iter()
            .map(|configuration| render::text(&survey.rule, configuration))
            .collect::<Vec<_>>();
        end.sort();
        found.sort();
        assert_eq!(found, end, "{source}");
        for kind in [Kind::End, Kind::Outcome] {
            for pattern in pattern {
                let claim = claim(kind, pattern);
                assert_eq!(
                    claim::survey(&claim, &survey).unwrap().answer,
                    claim::evaluate(&claim, &recorded).unwrap().answer,
                    "{source} {kind:?} {pattern}"
                );
            }
        }
    }
    let task = survey("Task.T1.Pending, Task.T2.Pending, [Pending] Running, [Running] Done");
    let whole = claim::survey(&exact(Kind::End, "Task.T1.Done, Task.T2.Done"), &task).unwrap();
    assert_eq!(whole.answer, Answer::Holds);
    let stray = claim::survey(&exact(Kind::Outcome, "Task.T1.Done"), &task).unwrap();
    assert_eq!(stray.answer, Answer::Fails);
    assert!(stray.witness.is_none());
    assert!(claim::survey(&claim(Kind::Reach, "Done"), &task).is_err());
}
