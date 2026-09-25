use crate::catalog::LIBRARY;
use crate::fixture::{Fixture, Value, digit, reach, value};
use crate::{check, witness};
use photonic::prism::Outcome;

fn library() -> Vec<&'static str> {
    LIBRARY.iter().map(|entry| entry.source.as_str()).collect()
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
            "().([1] 2)",
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
        "Push.([Digit] 1).Zero, Stage.1,
        [Built, Stage.1] (Operand.Left, Forget.Stage.2),
        [Clean.Stage.2] (Push.([Digit] 2).Zero, Stage.3),
        [Built, Stage.3] (Operand.Right, Forget.Stage.4),
        [Clean.Stage.4] Function.Natural.Add,
        [Return.Natural.Add] (Read, Forget.Inspect.1),
        [Yield.([Digit] 0), Clean.Inspect.1] (Read, Forget.Inspect.2),
        [Yield.([Digit] 1).Zero, Clean.Inspect.2] Done",
        "Done",
        &library(),
    );
}

#[test]
fn sort() {
    let item = [7, 0, 25, 7, 3].map(digit);
    let mut expected = item.to_vec();
    expected.sort_by_key(|digit| value(digit));
    let mut fixture = Fixture::new();
    let made = fixture.stage();
    let built = fixture.stage();
    let check = fixture.stage();
    fixture.vector(&item.map(Value::Natural), fixture.start(), &made, &built);
    fixture.rule(format!("[Clean.{built}, {made}] Function.Vector.Sort"));
    fixture.rule(format!("[Return.Vector.Sort] {check}"));
    fixture.inspect(
        &Value::Vector(expected.into_iter().map(Value::Natural).collect()),
        &check,
        "Done",
    );
    reach(&fixture.source(), "Done", &library(), "sort");
}
