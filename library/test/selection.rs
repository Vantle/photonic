use crate::catalog::source;
use crate::{check, witness};
use photonic::prism::Outcome;

fn library() -> [&'static str; 8] {
    [
        source("function", "invoke"),
        source("collection", "produce"),
        source("collection", "copy"),
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
