use crate::lowering::parse;
use crate::obsidian::{Outcome, Search};

fn check(source: &str, target: &str, expected: Outcome) {
    let mut search = Search::new(parse(source).unwrap(), parse(target).unwrap()).unwrap();
    search.run(50_000, None);
    let report = search.report();
    assert!(report.execution.closed, "{source}");
    assert_eq!(report.outcome, expected, "{source} => {target}");
}

#[test]
fn addition() {
    check(
        "Add(Unit.Unit.Unit,Unit.Unit.Unit.Unit.Unit.Unit.Unit) [Add,Add] ()",
        &["Unit"; 10].join("."),
        Outcome::Reached,
    );
    check(
        "Add(Unit.Unit.Unit,Unit.Unit.Unit.Unit.Unit.Unit.Unit.Unit) [Add,Add] ()",
        &["Unit"; 9].join("."),
        Outcome::Unreachable,
    );
}

#[test]
fn pairing() {
    let source = "Pair(A,B,C,D) [Pair,Pair] Result";
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
        "First(A,B),Second(C,D) [First,First] Result [Second,Second] Result",
        "Result.A.B,Result.C.D",
        Outcome::Reached,
    );
    check(
        "First(A,B),Second(C,D) [First,First] Result [Second,Second] Result",
        "Result.A.C,Result.B.D",
        Outcome::Unreachable,
    );
}

#[test]
fn provenance() {
    check("A(X,X)", "A.X,A.X", Outcome::Reached);
    check("Seed.X [Seed] A((),()) [A,A] ()", "X", Outcome::Reached);
    check(
        "Seed.X [Seed] A((),()) [A,A] ()",
        "X.X",
        Outcome::Unreachable,
    );
    check("A(X,X) [A,A] ()", "X.X", Outcome::Reached);
    check("A(X,X) [A,A] ()", "X", Outcome::Unreachable);
}

#[test]
fn evidence() {
    for count in 0..=4 {
        let input = std::iter::once("Check")
            .chain(std::iter::repeat_n("Unit", count))
            .collect::<Vec<_>>()
            .join(".");
        let rule = "[Check] Number [Number.Unit] Number";
        check(&format!("{input} {rule}"), "Number", Outcome::Reached);
        check(
            &format!("{input}.Extra {rule}"),
            "Number",
            Outcome::Unreachable,
        );
        check(
            &format!("{input}.Extra {rule}"),
            "Number.Extra",
            Outcome::Reached,
        );
    }
    check("Unit.Extra [Unit.Extra] Number", "Number", Outcome::Reached);
    check(
        "Pair(Seed,Other) [Seed] Kind [Other] Kind [Pair(Kind,Kind)] ([] Result)",
        "Result.Seed.Other",
        Outcome::Reached,
    );
    check(
        "Pair(Seed.Extra,Other) [Seed] Kind [Other] Kind [Pair(Kind,Kind)] ([] Result)",
        "Result.Seed.Extra.Other",
        Outcome::Reached,
    );
}

#[test]
fn example() {
    for (source, target, expected) in [
        (
            include_str!("../../example/group/addition.wave"),
            include_str!("../../mathematics/natural/ten.particle"),
            Outcome::Reached,
        ),
        (
            include_str!("../../example/group/pair.wave"),
            include_str!("../../example/group/selected.particle"),
            Outcome::Reached,
        ),
        (
            include_str!("../../example/group/isolated.wave"),
            include_str!("../../example/group/separate.particle"),
            Outcome::Reached,
        ),
        (
            include_str!("../../example/group/isolated.wave"),
            include_str!("../../example/group/crossed.particle"),
            Outcome::Unreachable,
        ),
        (
            include_str!("../../example/group/natural.wave"),
            include_str!("../../example/group/number.particle"),
            Outcome::Reached,
        ),
        (
            include_str!("../../example/group/extra.wave"),
            include_str!("../../example/group/number.particle"),
            Outcome::Unreachable,
        ),
        (
            include_str!("../../example/group/abstraction.wave"),
            include_str!("../../example/group/concrete.particle"),
            Outcome::Reached,
        ),
        (
            include_str!("../../example/group/broadcast.wave"),
            include_str!("../../example/group/shared.particle"),
            Outcome::Reached,
        ),
        (
            include_str!("../../example/group/broadcast.wave"),
            include_str!("../../example/group/double.particle"),
            Outcome::Unreachable,
        ),
        (
            include_str!("../../example/group/independent.wave"),
            include_str!("../../example/group/double.particle"),
            Outcome::Reached,
        ),
        (
            include_str!("../../example/group/independent.wave"),
            include_str!("../../example/group/shared.particle"),
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
        "Enter [Seed] Pair(A,B) [Enter] (Seed [Pair,Pair] Result)",
        "Result.A.B",
        Outcome::Reached,
    );
}
