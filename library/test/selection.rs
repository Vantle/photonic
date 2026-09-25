use crate::catalog::source;
use crate::{answer, check, witness};
use photonic::prism::Outcome;

fn library() -> [&'static str; 9] {
    [
        source("function", "invoke"),
        source("collection", "produce"),
        source("collection", "copy"),
        source("collection", "unpack"),
        source("collection", "map"),
        source("selection", "filter"),
        source("selection", "check"),
        source("selection", "count"),
        source("selection", "reduce"),
    ]
}

#[test]
fn count() {
    for (value, expected) in [("True", "2"), ("False", "0")] {
        witness(
            &format!(
                "Invoke.Pair.Copy.([Value] {value}).Map.([Each] Selection.Filter).Reduce.([Operation] Selection.Count)"
            ),
            expected,
            &library(),
        );
    }
}

#[test]
fn mixed() {
    for (left, right) in [("True", "False"), ("False", "True")] {
        answer(
            &format!(
                "Invoke.Pair.Unpack.([Left] {left}).([Right] {right}).Map.([Each] Selection.Filter).Reduce.([Operation] Selection.Count)"
            ),
            "1",
            ["0", "1", "2"],
            &library(),
        );
    }
}

#[test]
fn verdict() {
    for (value, verdict, other) in [
        ("True", "Success", "Failure"),
        ("False", "Failure", "Success"),
    ] {
        let source = format!("Invoke.Selection.Check.{value}");
        check(&source, verdict, &library(), Outcome::Reached);
        check(&source, other, &library(), Outcome::Unreachable);
    }
}
