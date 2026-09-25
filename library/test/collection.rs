use crate::catalog::source;
use crate::{answer, check, witness};
use photonic::prism::Outcome;

fn invoke() -> &'static str {
    source("function", "invoke")
}

fn collection(name: &str) -> &'static str {
    source("collection", name)
}

#[test]
fn freshness() {
    let produce = "Function.Pair.Produce.([Value] True), Function.Pair.Produce.([Value] True)";
    let reunion = "[Return.True, Return.True] True.True";
    check(
        produce,
        "True.True",
        &[collection("produce"), reunion],
        Outcome::Reached,
    );
    check(
        produce,
        "True",
        &[collection("produce"), reunion],
        Outcome::Unreachable,
    );
    let routing = "[Ready.Left, Ready.Right] ()";
    check(
        "Function.Pair.Broadcast.True",
        "True",
        &[collection("broadcast"), routing],
        Outcome::Reached,
    );
    check(
        "Function.Pair.Broadcast.True",
        "True.True",
        &[collection("broadcast"), routing],
        Outcome::Unreachable,
    );
    witness(
        "Invoke.Pair.Copy.([Value] True).Map.([Each] Identity).Gather",
        "([Left] True).([Right] True)",
        &[
            invoke(),
            source("function", "identity"),
            collection("produce"),
            collection("copy"),
            collection("map"),
            collection("gather"),
        ],
    );
}

#[test]
fn repetition() {
    witness(
        "Invoke.Pair.Repeat.([Value] 1).([Each] Pair.Produce).Reduce.([Operation] Ternary.Add)",
        "([Digit] 2).([Carry] 0)",
        &[
            invoke(),
            collection("produce"),
            collection("repeat"),
            collection("map"),
            collection("reduce"),
            source("ternary", "add"),
        ],
    );
}

#[test]
fn order() {
    for left in ["True", "False"] {
        for right in ["True", "False"] {
            for condition in ["True", "False"] {
                answer(
                    &format!("Invoke.Pair.Choose.{condition}.([Left] {left}).([Right] {right})"),
                    if condition == "True" { left } else { right },
                    ["True", "False"],
                    &[invoke(), collection("choose")],
                );
            }
        }
    }
}

#[test]
fn completion() {
    let library = [invoke(), collection("gather")];
    check(
        "Invoke.Gather.Done.Left.True",
        "([Left] True).([Right] True)",
        &library,
        Outcome::Unreachable,
    );
    check(
        "Invoke.Gather.Done.Left.True, Invoke.Gather.Done.Right.True",
        "([Left] True).([Right] True)",
        &library,
        Outcome::Unreachable,
    );
    for left in ["True", "False"] {
        for right in ["True", "False"] {
            witness(
                &format!(
                    "Invoke.Pair.Unpack.([Left] {left}).([Right] {right}).Map.([Each] Identity).Gather"
                ),
                &format!("([Left] {left}).([Right] {right})"),
                &[
                    invoke(),
                    source("function", "identity"),
                    collection("unpack"),
                    collection("map"),
                    collection("gather"),
                ],
            );
        }
    }
    check(
        "Function.Pair.Produce.([Value] True), Function.Pair.Produce.([Value] True)",
        "True",
        &[collection("produce"), "[Return, Return] ()"],
        Outcome::Reached,
    );
}

#[test]
fn empty() {
    for (source, target, library) in [
        ("Invoke.Empty.Repeat", "()", collection("repeat")),
        ("Invoke.Empty.Gather", "()", collection("gather")),
        (
            "Invoke.Empty.Reduce.([Operation] Boolean.And)",
            "True",
            source("boolean", "and"),
        ),
        (
            "Invoke.Empty.Reduce.([Operation] Boolean.Or)",
            "False",
            source("boolean", "or"),
        ),
        (
            "Invoke.Empty.Reduce.([Operation] Boolean.Equal)",
            "True",
            source("boolean", "equal"),
        ),
        (
            "Invoke.Empty.Reduce.([Operation] Ternary.Add)",
            "([Digit] 0).([Carry] 0)",
            source("ternary", "add"),
        ),
        (
            "Invoke.Empty.Reduce.([Operation] Selection.Count)",
            "0",
            source("selection", "count"),
        ),
    ] {
        check(source, target, &[invoke(), library], Outcome::Reached);
    }
}

#[test]
fn association() {
    let request =
        "Invoke.Twin.True.([Operation] Boolean.And), Invoke.Twin.False.([Operation] Boolean.And)";
    let library = [
        invoke(),
        source("boolean", "and"),
        collection("reduce"),
        "[Function.Twin.True] (Reduce.Done.Left.True, Reduce.Done.Right.True), [Function.Twin.False] (Reduce.Done.Left.False, Reduce.Done.Right.False)",
    ];
    for (target, expected) in [
        ("True, False", Outcome::Reached),
        ("True, True", Outcome::Unreachable),
        ("False, False", Outcome::Unreachable),
    ] {
        check(request, target, &library, expected);
    }
}

#[test]
fn extension() {
    let implementation = "[Invert.([Each] Invert).True] Return.False, [Invert.([Each] Invert).False] Return.True, [Agree.([Operation] Agree).True.True] Return.True, [Agree.([Operation] Agree).True.False] Return.False, [Agree.([Operation] Agree).False.False] Return.False";
    witness(
        "Invoke.Pair.Copy.([Value] False).Map.([Each] Invert).Reduce.([Operation] Agree)",
        "True",
        &[
            invoke(),
            collection("produce"),
            collection("copy"),
            collection("map"),
            collection("reduce"),
            implementation,
        ],
    );
}

#[test]
fn descriptor() {
    check(
        "Map.Ready.Left.True.([Function] Boolean.Not)",
        "Done.Left.False",
        &[invoke(), source("boolean", "not"), collection("map")],
        Outcome::Unreachable,
    );
}
