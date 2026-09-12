use molten::lowering::{self, Failure};
use molten::source::Value;

fn atom(value: &[&str]) -> Vec<Value> {
    value
        .iter()
        .map(|value| Value::Atom((*value).to_owned()))
        .collect()
}

#[test]
fn conjunction() {
    let source = lowering::parse(
        "And.True.False.Extra; [True] -> Boolean; [False] -> Boolean; [And.Boolean.Boolean] -> { [True.False] -> False; };",
    )
    .unwrap();
    assert_eq!(source.initial, [atom(&["And", "True", "False", "Extra"])]);
    assert_eq!(source.rule.len(), 3);
    assert_eq!(source.rule[2].input, [atom(&["And", "Boolean", "Boolean"])]);
    let body = source.rule[2].output[0].body.as_ref().unwrap();
    assert!(source.rule[2].output[0].particle.is_empty());
    assert_eq!(body[0].input, [atom(&["True", "False"])]);
    assert_eq!(body[0].output[0].particle, atom(&["False"]));
}

#[test]
fn coherence() {
    let source = lowering::parse("A.X, B.Y; [A, B] -> C, D; [C, D] -> E;").unwrap();
    assert_eq!(source.initial, [atom(&["A", "X"]), atom(&["B", "Y"])]);
    assert_eq!(source.rule[0].input, [atom(&["A"]), atom(&["B"])]);
    assert_eq!(source.rule[0].output.len(), 2);
    assert_eq!(source.rule[0].output[0].particle, atom(&["C"]));
    assert_eq!(source.rule[0].output[1].particle, atom(&["D"]));
}

#[test]
fn closure() {
    let source = lowering::parse("@([A] -> B).A; [@([A] -> B)] -> @([A] -> C);").unwrap();
    let Value::Rule { rule } = &source.initial[0][0] else {
        panic!("expected a rule value");
    };
    assert_eq!(rule.input, [atom(&["A"])]);
    assert_eq!(rule.output[0].particle, atom(&["B"]));
    let Value::Rule { rule } = &source.rule[0].output[0].particle[0] else {
        panic!("expected a replacement rule value");
    };
    assert_eq!(rule.output[0].particle, atom(&["C"]));
    let recursive = lowering::parse("@([@([A] -> B)] -> @([A] -> C));").unwrap();
    assert!(matches!(recursive.initial[0][0], Value::Rule { .. }));
}

#[test]
fn scope() {
    let source = lowering::parse(
        "Enter; [Enter] -> { Make; [Make] -> { Step; [Step] -> @([Call] -> { [Payload] -> Done; }); }; };",
    )
    .unwrap();
    let outer = &source.rule[0].output[0];
    assert_eq!(outer.particle, atom(&["Make"]));
    let inner = &outer.body.as_ref().unwrap()[0].output[0];
    assert_eq!(inner.particle, atom(&["Step"]));
    let Value::Rule { rule } = &inner.body.as_ref().unwrap()[0].output[0].particle[0] else {
        panic!("expected an escaped closure");
    };
    assert_eq!(
        rule.output[0].body.as_ref().unwrap()[0].input,
        [atom(&["Payload"])]
    );
}

#[test]
fn absence() {
    let source =
        lowering::parse("Start; [Start] unless [Q] -> P; [P] unless [@([A] -> B), Q] -> R;")
            .unwrap();
    assert_eq!(source.rule[0].negative, Some(vec![atom(&["Q"])]));
    let negative = source.rule[1].negative.as_ref().unwrap();
    assert_eq!(negative.len(), 2);
    assert!(matches!(negative[0][0], Value::Rule { .. }));
    assert_eq!(negative[1], atom(&["Q"]));
}

#[test]
fn empty() {
    let source = lowering::parse("(); [()] -> A; [A] -> (); [A] -> []; [] -> A;").unwrap();
    assert_eq!(source.initial, [Vec::<Value>::new()]);
    assert_eq!(source.rule[0].input, [Vec::<Value>::new()]);
    assert_eq!(source.rule[1].output.len(), 1);
    assert!(source.rule[1].output[0].particle.is_empty());
    assert!(source.rule[2].output.is_empty());
    assert!(source.rule[3].input.is_empty());
    let source = lowering::parse("[A] -> {};").unwrap();
    assert!(source.initial.is_empty());
    assert_eq!(source.rule[0].output[0].body.as_ref().unwrap().len(), 0);
    let source = lowering::parse("").unwrap();
    assert!(source.initial.is_empty() && source.rule.is_empty());
}

#[test]
fn unicode() {
    let source = lowering::parse("人.世界; [人] -> 🌋;").unwrap();
    assert_eq!(source.initial, [atom(&["人", "世界"])]);
    assert_eq!(source.rule[0].output[0].particle, atom(&["🌋"]));
    let Failure::Syntax { span, .. } = lowering::parse("人]").unwrap_err() else {
        panic!("expected a syntax diagnostic");
    };
    assert_eq!(span.offset(), 3);
    assert_eq!(span.len(), 1);
    let Failure::Syntax { span, .. } = lowering::parse("[人] ->").unwrap_err() else {
        panic!("expected an end-of-input diagnostic");
    };
    assert_eq!(span.offset(), "[人] ->".len());
    assert_eq!(span.len(), 0);
}

#[test]
fn malformed() {
    for source in [
        "A",
        "A..B;",
        "[A] ->;",
        "[A] -> B",
        "[A] unless -> B;",
        "[A] -> { [B] -> C;",
        "@([A] -> B;);",
        "[A] -> [], B;",
    ] {
        assert!(
            matches!(lowering::parse(source), Err(Failure::Syntax { .. })),
            "{source}"
        );
    }
    let source = "[A] -> { X, Y; [X] -> Z; };";
    let Failure::Body { count, span } = lowering::parse(source).unwrap_err() else {
        panic!("expected a body initialization diagnostic");
    };
    assert_eq!(count, 2);
    assert_eq!(
        &source[span.offset()..span.offset() + span.len()],
        "{ X, Y; [X] -> Z; }"
    );
}

#[test]
fn depth() {
    let source = "{".repeat(129);
    assert!(matches!(
        lowering::parse(&source),
        Err(Failure::Depth { limit: 128, .. })
    ));
    let source = format!("A; {}", "[A] -> A; ".repeat(10_000));
    assert_eq!(lowering::parse(&source).unwrap().rule.len(), 10_000);
}
