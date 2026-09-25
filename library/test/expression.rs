use crate::catalog::{LIBRARY, source};
use crate::{answer, witness};

fn library() -> Vec<&'static str> {
    LIBRARY
        .iter()
        .filter(|entry| {
            ["chain", "expression", "integer", "natural"].contains(&entry.package.as_str())
        })
        .map(|entry| entry.source.as_str())
        .collect()
}

fn tape(token: &[&str]) -> String {
    let Some((last, rest)) = token.split_last() else {
        return "Function.Expression.Execute.Zero".to_owned();
    };
    let mut rule = vec![format!("Push.{last}.Zero, Stage.0")];
    let mut stage = "Stage.0".to_owned();
    for (index, token) in rest.iter().rev().enumerate() {
        let next = format!("Stage.{}", index + 1);
        rule.push(format!("[Built, {stage}] (Push.{token}, Forget.{next})"));
        stage = format!("Clean.{next}");
    }
    rule.push(format!("[Built, {stage}] Function.Expression.Execute"));
    rule.join(",\n")
}

fn execute(token: &[&str], error: &str) {
    witness(
        &tape(token),
        &format!("Return.Expression.Execute.Error.{error}"),
        &library(),
    );
}

#[test]
fn invalid() {
    for token in [
        &["([Digit] 0)"][..],
        &["([Digit] 1)"][..],
        &["([Digit] 2)"][..],
        &["Mark"][..],
        &["Open"][..],
        &["Close"][..],
        &["Literal", "Literal"][..],
        &["Literal", "([Digit] 2)", "Add"][..],
        &["Literal", "Subtract"][..],
        &["Literal", "Multiply"][..],
        &["Literal", "Divide"][..],
        &["Literal", "Open"][..],
        &["Literal", "Close"][..],
        &["Literal", "([Digit] 1)"][..],
    ] {
        execute(token, "Syntax");
    }
}

#[test]
fn stack() {
    for operator in ["Add", "Subtract", "Multiply", "Divide"] {
        execute(&[operator], "Stack");
        execute(&["Literal", "([Digit] 1)", "Mark", operator], "Stack");
    }
    execute(&["Negate"], "Stack");
}

#[test]
fn forward() {
    let error = ["Syntax", "Stack", "Divisor"];
    for kind in error {
        let stub =
            format!("[Function.Expression.Execute.Zero] Return.Expression.Execute.Error.{kind}");
        answer(
            "Function.Expression.Evaluate.Zero",
            &format!("Return.Expression.Evaluate.Error.{kind}"),
            error.map(|other| format!("Return.Expression.Evaluate.Error.{other}")),
            &[
                source("chain", "cell"),
                source("expression", "evaluate"),
                "[Function.Expression.Parse.Zero] Return.Expression.Parse.Program.Zero",
                stub.as_str(),
            ],
        );
    }
}
