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
    let source = lowering::parse(include_str!("../../../example/conjunction.lava")).unwrap();
    assert_eq!(source.initial, [atom(&["And", "True", "False", "Extra"])]);
    assert_eq!(source.rule.len(), 3);
    assert_eq!(source.rule[2].input, [atom(&["And", "Boolean", "Boolean"])]);
    let body = source.rule[2].output[0].body.as_ref().unwrap();
    assert!(source.rule[2].output[0].particle.is_empty());
    assert_eq!(body[1].input, [atom(&["True", "False"])]);
    assert_eq!(body[1].output[0].particle, atom(&["False"]));
}

#[test]
fn coherence() {
    let source = lowering::parse("A.X, B.Y [A, B] (C, D) [C, D] E").unwrap();
    assert_eq!(source.initial, [atom(&["A", "X"]), atom(&["B", "Y"])]);
    assert_eq!(source.rule[0].input, [atom(&["A"]), atom(&["B"])]);
    assert_eq!(source.rule[0].output.len(), 2);
    assert_eq!(source.rule[0].output[0].particle, atom(&["C"]));
    assert_eq!(source.rule[0].output[1].particle, atom(&["D"]));
    assert_eq!(
        lowering::parse("[A, B, C] (A) (B) (C),").unwrap().rule[0]
            .output
            .len(),
        3
    );
}

#[test]
fn closure() {
    let source = lowering::parse("([A] B).A [[A] B] [A] C").unwrap();
    let Value::Rule { rule } = &source.initial[0][0] else {
        panic!("expected rule");
    };
    assert_eq!(rule.input, [atom(&["A"])]);
    assert_eq!(rule.output[0].particle, atom(&["B"]));
    let Value::Rule { rule } = &source.rule[0].output[0].particle[0] else {
        panic!("expected rule");
    };
    assert_eq!(rule.output[0].particle, atom(&["C"]));
    let source = lowering::parse("[[A,B]] ([B])").unwrap();
    let Value::Rule { rule } = &source.rule[0].input[0][0] else {
        panic!("expected rule");
    };
    assert_eq!(rule.input, [atom(&["A"]), atom(&["B"])]);
    assert!(rule.output.is_empty());
    assert_eq!(
        source.rule[0].output[0].body.as_ref().unwrap()[0].input,
        [atom(&["B"])]
    );
}

#[test]
fn scope() {
    let source =
        lowering::parse("Enter [Enter] (Make [Make] [Call] (Payload [Payload] Done))").unwrap();
    let outer = &source.rule[0].output[0];
    assert_eq!(outer.particle, atom(&["Make"]));
    let Value::Rule { rule } = &outer.body.as_ref().unwrap()[0].output[0].particle[0] else {
        panic!("expected rule");
    };
    assert_eq!(
        rule.output[0].body.as_ref().unwrap()[0].input,
        [atom(&["Payload"])]
    );
}

#[test]
fn empty() {
    let source = lowering::parse("() [()] A, [A] (), [A], [] A").unwrap();
    assert_eq!(source.initial, [Vec::<Value>::new()]);
    assert_eq!(source.rule[0].input, [Vec::<Value>::new()]);
    assert_eq!(source.rule[1].output.len(), 1);
    assert!(source.rule[1].output[0].particle.is_empty());
    assert!(source.rule[2].output.is_empty());
    assert!(source.rule[3].input.is_empty());
    let source = lowering::parse("").unwrap();
    assert!(source.initial.is_empty() && source.rule.is_empty());
}

#[test]
fn alphabet() {
    let source = lowering::parse("$x.@.unless.->.;.{.}").unwrap();
    assert_eq!(
        source.initial,
        [atom(&["$x", "@", "unless", "->", ";", "{", "}"])]
    );
    let source = lowering::parse("Box(A.B)").unwrap();
    assert_eq!(source.initial, [atom(&["Box", "A", "B"])]);
    let source = lowering::parse("[Box(A)] Box(B)").unwrap();
    assert_eq!(source.rule[0].input, [atom(&["Box", "A"])]);
    assert_eq!(source.rule[0].output[0].particle, atom(&["Box", "B"]));
    assert!(serde_json::from_str::<Value>(r#"{"variable":"x"}"#).is_err());
    assert!(serde_json::from_str::<Value>(r#"{"structure":"Box","particle":["A"]}"#).is_err());
}

#[test]
fn unicode() {
    let source = lowering::parse("人.世界 [人] 🌋").unwrap();
    assert_eq!(source.initial, [atom(&["人", "世界"])]);
    assert_eq!(source.rule[0].output[0].particle, atom(&["🌋"]));
    let Failure::Syntax { span, .. } = lowering::parse("人]").unwrap_err() else {
        panic!("expected diagnostic");
    };
    assert_eq!(span.offset(), 3);
    assert_eq!(span.len(), 1);
}

#[test]
fn malformed() {
    for source in ["A..B", ".A", "[A] B.", "[A B] C", "[A] (B,C [B] D)"] {
        assert!(lowering::parse(source).is_err(), "{source}");
    }
    assert!(matches!(
        lowering::parse(&"[".repeat(129)),
        Err(Failure::Depth { limit: 128, .. })
    ));
    assert!(lowering::parse(&format!("{}A{}", "(".repeat(128), ")".repeat(128))).is_ok());
}

#[test]
fn grouping() {
    let compact = lowering::parse("[A] (B).C").unwrap();
    let spaced = lowering::parse("[A] (B) . C").unwrap();
    assert_eq!(compact.rule[0].canonical(), spaced.rule[0].canonical());
}

#[test]
fn schema() {
    for source in [
        r#"{"rule":[{"input":[["A"]],"output":[{"particle":["B"]}],"negative":[["C"]]}]}"#,
        r#"{"initial":[[{"rule":{"input":[["A"]],"output":[]},"negative":[["C"]]}]]}"#,
        r#"{"initial":[[{"rule":{"input":[["A"]],"output":[],"negative":[["C"]]}}]]}"#,
    ] {
        assert!(
            serde_json::from_str::<molten::source::Program>(source).is_err(),
            "{source}"
        );
    }
}

#[test]
fn recursion() {
    assert!(lowering::parse(&"[A] ".repeat(128)).is_ok());
    for count in [129, 10_000] {
        let error = lowering::parse(&"[A] ".repeat(count)).unwrap_err();
        let Failure::Depth { limit, span } = error else {
            panic!("expected depth diagnostic");
        };
        assert_eq!(limit, 128);
        assert_eq!(span.offset(), 128 * 4);
    }
    let mixed = format!("{}({})", "[A] ".repeat(127), "[B] ".repeat(2));
    assert!(matches!(
        lowering::parse(&mixed),
        Err(Failure::Depth { .. })
    ));
    assert!(lowering::parse(&"[A] B, ".repeat(10_000)).is_ok());
}
