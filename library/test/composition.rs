use crate::catalog::LIBRARY;
use crate::{check, witness};
use photonic::prism::Outcome;

fn library() -> Vec<&'static str> {
    LIBRARY.iter().map(|entry| entry.2).collect()
}

#[test]
fn scalar() {
    for (source, target, expected) in [
        ("Invoke.Boolean.And.True.False", "False", Outcome::Reached),
        (
            "Invoke.Boolean.And.True.False",
            "True",
            Outcome::Unreachable,
        ),
        (
            "Invoke.Ternary.Multiply.2.2",
            "([Digit] 1).([Carry] 1)",
            Outcome::Reached,
        ),
        ("Invoke.Binary.Multiply.1.1", "1", Outcome::Reached),
        (
            "Invoke.Binary.Multiply.1.1",
            "([Digit] 1).([Carry] 0)",
            Outcome::Unreachable,
        ),
        (
            "Invoke.Binary.Sum.1.1.0",
            "([Digit] 2).([Carry] 0)",
            Outcome::Unreachable,
        ),
        (
            "Invoke.Carry.Combine.([Left] Kill).([Right] Generate)",
            "Generate",
            Outcome::Reached,
        ),
        (
            "Invoke.Field.Pack.([Position] 1).([Value] 2)",
            "([1] 2)",
            Outcome::Reached,
        ),
    ] {
        check(source, target, &library(), expected);
    }
}

#[test]
fn pipeline() {
    witness(
        include_str!("../../program/composition/pipeline.wave"),
        include_str!("../../program/composition/pipeline.particle"),
        &library(),
    );
}

#[test]
fn linked() {
    witness(
        "Push.([Digit] 1).Zero, Stage.1
        [Built, Stage.1] (Operand.Left) (Forget.Stage.2)
        [Clean.Stage.2] (Push.([Digit] 2).Zero) (Stage.3)
        [Built, Stage.3] (Operand.Right) (Forget.Stage.4)
        [Clean.Stage.4] Function.Natural.Add
        [Return.Natural.Add] (Read) (Forget.Inspect.1)
        [Yield.([Digit] 0), Clean.Inspect.1] (Read) (Forget.Inspect.2)
        [Yield.([Digit] 1).Zero, Clean.Inspect.2] Done",
        "Done",
        &library(),
    );
}
