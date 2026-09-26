use crate::prism::Outcome;
use crate::runtime::{Limit, Runtime};
use frontend::lowering::parse;

fn check(source: &str, target: &str, expected: Outcome) {
    let program = parse(source).unwrap();
    let goal = crate::test::target(&program, target);
    let mut runtime = Runtime::new(&program);
    runtime.run(50_000, Limit::default());
    let verdict = runtime.verdict(&goal);
    let report = runtime.snapshot();
    assert!(report.closed, "{source}");
    assert_eq!(verdict.outcome, expected, "{source} => {target}");
}

#[test]
fn addition() {
    check(
        "Add.(Unit.Unit.Unit,Unit.Unit.Unit.Unit.Unit.Unit.Unit), [Add,Add] ()",
        &["Unit"; 10].join("."),
        Outcome::Reached,
    );
    check(
        "Add.(Unit.Unit.Unit,Unit.Unit.Unit.Unit.Unit.Unit.Unit.Unit), [Add,Add] ()",
        &["Unit"; 9].join("."),
        Outcome::Unreachable,
    );
}

#[test]
fn pairing() {
    let source = "Pair.(A,B,C,D), [Pair,Pair] Result";
    let operand = ["A", "B", "C", "D"];
    for left in 0..4 {
        for right in left + 1..4 {
            let mut target = vec![format!("Result.{}.{}", operand[left], operand[right])];
            target.extend(
                operand
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != left && *index != right)
                    .map(|(_, value)| format!("Pair.{value}")),
            );
            check(source, &target.join(","), Outcome::Reached);
        }
    }
    check(
        "First.(A,B),Second.(C,D), [First,First] Result, [Second,Second] Result",
        "Result.A.B,Result.C.D",
        Outcome::Reached,
    );
    check(
        "First.(A,B),Second.(C,D), [First,First] Result, [Second,Second] Result",
        "Result.A.C,Result.B.D",
        Outcome::Unreachable,
    );
}

#[test]
fn scope() {
    check("Z, (X, [X] Y)", "Z, Y", Outcome::Reached);
    check("Z, (X, [X] Y)", "Z, X", Outcome::Unreachable);
    check("A.X, [A] (B, C, [B, C] D)", "D.X", Outcome::Reached);
    check("A.X, [A] (B, C, [B, C] D)", "D.X.X", Outcome::Unreachable);
    check("Go, [Go] ((C, [C] D), [D] E)", "E", Outcome::Reached);
    check(
        "Go, [Go] ((), (C, [C] D), [D] E)",
        "E",
        Outcome::Unreachable,
    );
    for (source, count) in [
        ("(X, [X] Y), (X, [X] Y)", 3),
        ("A, [A] ((B, [B] C), (B, [B] C))", 4),
    ] {
        let mut runtime = Runtime::new(&parse(source).unwrap());
        runtime.run(50_000, Limit::default());
        assert_eq!(runtime.snapshot().state.len(), count, "{source}");
    }
    check("Z, (X, [X] Y)", "Z, (X, [X] Y)", Outcome::Reached);
    check("Z, (X, [X] Y)", "Z, (X, [X] W)", Outcome::Unreachable);
    check(
        "A, [A] (X, [X] Y)",
        "(X, [X] Y), [A] (X, [X] Y)",
        Outcome::Unreachable,
    );
    check("(X, [X] Y), (X, [X] Y)", "Y, (X, [X] Y)", Outcome::Reached);
}

#[test]
fn provenance() {
    check("A.(X,X)", "A.X,A.X", Outcome::Reached);
    check("Seed.X, [Seed] A.((),()), [A,A] ()", "X", Outcome::Reached);
    check(
        "Seed.X, [Seed] A.((),()), [A,A] ()",
        "X.X",
        Outcome::Unreachable,
    );
    check("A.(X,X), [A,A] ()", "X.X", Outcome::Reached);
    check("A.(X,X), [A,A] ()", "X", Outcome::Unreachable);
}

#[test]
fn evidence() {
    for count in 0..=4 {
        let input = std::iter::once("Check")
            .chain(std::iter::repeat_n("Unit", count))
            .collect::<Vec<_>>()
            .join(".");
        let rule = "[Check] Number, [Number.Unit] Number";
        check(&format!("{input}, {rule}"), "Number", Outcome::Reached);
        check(
            &format!("{input}.Extra, {rule}"),
            "Number",
            Outcome::Unreachable,
        );
        check(
            &format!("{input}.Extra, {rule}"),
            "Number.Extra",
            Outcome::Reached,
        );
    }
    check(
        "Unit.Extra, [Unit.Extra] Number",
        "Number",
        Outcome::Reached,
    );
    check(
        "Pair.(Seed,Other), [Seed] Kind, [Other] Kind, [Pair.(Kind,Kind)] ([()] Result)",
        "Result.Seed.Other",
        Outcome::Reached,
    );
    check(
        "Pair.(Seed.Extra,Other), [Seed] Kind, [Other] Kind, [Pair.(Kind,Kind)] ([()] Result)",
        "Result.Seed.Extra.Other",
        Outcome::Reached,
    );
}

#[test]
fn example() {
    for (source, target, expected) in [
        (
            include_str!("../../program/association/addition.wave"),
            include_str!("../../program/natural/ten.particle"),
            Outcome::Reached,
        ),
        (
            include_str!("../../program/association/pair.wave"),
            include_str!("../../program/association/selected.particle"),
            Outcome::Reached,
        ),
        (
            include_str!("../../program/association/isolated.wave"),
            include_str!("../../program/association/separate.particle"),
            Outcome::Reached,
        ),
        (
            include_str!("../../program/association/isolated.wave"),
            include_str!("../../program/association/crossed.particle"),
            Outcome::Unreachable,
        ),
        (
            include_str!("../../program/association/natural.wave"),
            include_str!("../../program/association/number.particle"),
            Outcome::Reached,
        ),
        (
            include_str!("../../program/association/extra.wave"),
            include_str!("../../program/association/number.particle"),
            Outcome::Unreachable,
        ),
        (
            include_str!("../../program/association/abstraction.wave"),
            include_str!("../../program/association/concrete.particle"),
            Outcome::Reached,
        ),
        (
            include_str!("../../program/association/broadcast.wave"),
            include_str!("../../program/association/shared.particle"),
            Outcome::Reached,
        ),
        (
            include_str!("../../program/association/broadcast.wave"),
            include_str!("../../program/association/double.particle"),
            Outcome::Unreachable,
        ),
        (
            include_str!("../../program/association/independent.wave"),
            include_str!("../../program/association/double.particle"),
            Outcome::Reached,
        ),
        (
            include_str!("../../program/association/independent.wave"),
            include_str!("../../program/association/shared.particle"),
            Outcome::Unreachable,
        ),
    ] {
        check(source, target, expected);
    }
}

#[test]
fn code() {
    check("([A] B).(A,A)", "([A] B).B,([A] B).B", Outcome::Reached);
    check(
        "Enter, [Seed] Pair.(A,B), [Enter] (Seed, [Pair,Pair] Result)",
        "Result.A.B",
        Outcome::Reached,
    );
}
