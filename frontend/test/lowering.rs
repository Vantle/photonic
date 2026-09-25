use frontend::lowering::{self, Failure};
use frontend::source::{Program, Value};

fn atom(value: &[&str]) -> Vec<Value> {
    value
        .iter()
        .map(|value| Value::Atom((*value).to_owned()))
        .collect()
}

fn same(left: &str, right: &str) {
    let canonical = |source: &str| {
        let program = lowering::parse(source).unwrap();
        let mut initial = program.initial;
        initial.iter_mut().for_each(|particle| particle.sort());
        initial.sort();
        let mut rule = program
            .rule
            .iter()
            .map(|rule| rule.canonical())
            .collect::<Vec<_>>();
        rule.sort();
        (initial, rule)
    };
    assert_eq!(canonical(left), canonical(right), "{left} and {right}");
}

fn syntax(source: &str) -> String {
    match lowering::parse(source) {
        Err(
            Failure::Syntax { message, .. }
            | Failure::Parse(frontend::failure::Failure::Syntax { message, .. }),
        ) => message,
        other => panic!("expected a syntax failure for {source}, found {other:?}"),
    }
}

#[test]
fn conjunction() {
    let source = lowering::parse(include_str!("../../program/language/conjunction.wave")).unwrap();
    assert_eq!(source.initial, [atom(&["And", "True", "False", "Extra"])]);
    assert_eq!(source.rule.len(), 3);
    assert_eq!(source.rule[2].input, [atom(&["And", "Boolean", "Boolean"])]);
    let body = source.rule[2].output[0].body.as_ref().unwrap();
    assert!(source.rule[2].output[0].particle.is_empty());
    assert_eq!(body[1].input, [atom(&["True", "False"])]);
    assert_eq!(body[1].output[0].particle, atom(&["False"]));
}

#[test]
fn partition() {
    let source = lowering::parse("A.X, B.Y, [A, B] (C, D), [C, D] E").unwrap();
    assert_eq!(source.initial, [atom(&["A", "X"]), atom(&["B", "Y"])]);
    assert_eq!(source.rule[0].input, [atom(&["A"]), atom(&["B"])]);
    assert_eq!(source.rule[0].output.len(), 2);
    assert_eq!(source.rule[0].output[0].particle, atom(&["C"]));
    assert_eq!(source.rule[0].output[1].particle, atom(&["D"]));
    let source = lowering::parse("[1] 1, [2.3] 6, [3.7] 21, 3.7").unwrap();
    assert_eq!(source.initial, [atom(&["3", "7"])]);
    assert_eq!(source.rule.len(), 3);
    assert_eq!(source.rule[2].output.len(), 1);
    same(
        "[1] 1, [2.3] 6, [3.7] 21, 3.7",
        "3.7, [3.7] 21, [2.3] 6, [1] 1",
    );
    same("A,\n[A] B,\nC,", "C, [A] B, A");
    same("A, B,", "A, B");
    let source = lowering::parse("[X] (A.B, C)").unwrap();
    assert_eq!(source.rule[0].output.len(), 2);
    assert_eq!(source.rule[0].output[0].particle, atom(&["A", "B"]));
    assert_eq!(source.rule[0].output[1].particle, atom(&["C"]));
}

#[test]
fn space() {
    for source in [
        "A B",
        "A.B C",
        "A(B, C)",
        "[A] B C",
        "[A] (B) (C)",
        "C [A] B",
        "C B [A]",
        "[A B] C",
        "(Kettle [Kettle.Tea] Cup)",
        "[A] B\n[B] C",
    ] {
        let message = syntax(source);
        assert!(
            message.contains("dot") && message.contains("comma"),
            "{source}: {message}"
        );
    }
    for source in ["X.[A] B", "[A].B"] {
        assert!(syntax(source).contains("parentheses"), "{source}");
    }
    for source in [",A", "A,,B", "(,)", "[,] A", "[A,,]"] {
        assert!(syntax(source).contains("comma"), "{source}");
    }
    same("A . B", "A.B");
    same("A.(B, C)", "A.B, A.C");
}

#[test]
fn closure() {
    let source = lowering::parse("([A] B).A, [[A] B] ().([A] C)").unwrap();
    let Value::Rule { rule } = &source.initial[0][0] else {
        panic!("expected rule");
    };
    assert_eq!(rule.input, [atom(&["A"])]);
    assert_eq!(rule.output[0].particle, atom(&["B"]));
    let Value::Rule { rule } = &source.rule[0].output[0].particle[0] else {
        panic!("expected rule");
    };
    assert_eq!(rule.output[0].particle, atom(&["C"]));
    assert!(source.rule[0].output[0].body.is_none());
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
    let source = lowering::parse("().([A] B), [A] B").unwrap();
    assert!(
        matches!(&source.initial[..], [particle] if matches!(&particle[..], [Value::Rule { .. }]))
    );
    assert_eq!(source.rule.len(), 1);
}

#[test]
fn scope() {
    let source =
        lowering::parse("Enter, [Enter] (Make, [Make] ().([Call] (Payload, [Payload] Done)))")
            .unwrap();
    let outer = &source.rule[0].output[0];
    assert_eq!(outer.particle, atom(&["Make"]));
    let Value::Rule { rule } = &outer.body.as_ref().unwrap()[0].output[0].particle[0] else {
        panic!("expected rule");
    };
    assert_eq!(
        rule.output[0].body.as_ref().unwrap()[0].input,
        [atom(&["Payload"])]
    );
    let source = lowering::parse("[A] (B, (X, [X] Y))").unwrap();
    assert_eq!(source.rule[0].output.len(), 2);
    assert!(source.rule[0].output[0].body.is_none());
    assert_eq!(source.rule[0].output[1].particle, atom(&["X"]));
    assert_eq!(source.rule[0].output[1].body.as_ref().unwrap().len(), 1);
    let source = lowering::parse("[A] ([B] C, [C] D,)").unwrap();
    assert!(source.rule[0].output[0].particle.is_empty());
    assert_eq!(source.rule[0].output[0].body.as_ref().unwrap().len(), 2);
    let source = lowering::parse("[A] (X, [X] Y)").unwrap();
    assert_eq!(source.rule[0].output[0].particle, atom(&["X"]));
    assert_eq!(source.rule[0].output[0].body.as_ref().unwrap().len(), 1);
    let source = lowering::parse("[A] X.([X] Y)").unwrap();
    assert!(source.rule[0].output[0].body.is_none());
    assert!(matches!(
        source.rule[0].output[0].particle[..],
        [Value::Atom(_), Value::Rule { .. }]
    ));
}

#[test]
fn empty() {
    let source = lowering::parse("(), [()] A, [A] (), [A], [] A, [(), ()] B").unwrap();
    assert_eq!(source.initial, [Vec::<Value>::new()]);
    assert_eq!(source.rule[0].input, [Vec::<Value>::new()]);
    assert_eq!(source.rule[1].output.len(), 1);
    assert!(source.rule[1].output[0].particle.is_empty());
    assert!(source.rule[2].output.is_empty());
    assert!(source.rule[3].input.is_empty());
    assert_eq!(source.rule[4].input.len(), 2);
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
    let source = lowering::parse("Box.(A.B)").unwrap();
    assert_eq!(source.initial, [atom(&["Box", "A", "B"])]);
    let source = lowering::parse("[Box.(A)] Box.(B)").unwrap();
    assert_eq!(source.rule[0].input, [atom(&["Box", "A"])]);
    assert_eq!(source.rule[0].output[0].particle, atom(&["Box", "B"]));
    assert!(serde_json::from_str::<Value>(r#"{"variable":"x"}"#).is_err());
    assert!(serde_json::from_str::<Value>(r#"{"structure":"Box","particle":["A"]}"#).is_err());
}

#[test]
fn unicode() {
    let source = lowering::parse("人.世界, [人] 🌋").unwrap();
    assert_eq!(source.initial, [atom(&["人", "世界"])]);
    assert_eq!(source.rule[0].output[0].particle, atom(&["🌋"]));
    let Failure::Parse(frontend::failure::Failure::Syntax { span, .. }) =
        lowering::parse("人]").unwrap_err()
    else {
        panic!("expected diagnostic");
    };
    assert_eq!(span.offset(), 3);
    assert_eq!(span.len(), 1);
}

#[test]
fn malformed() {
    for source in ["A..B", ".A", "[A] B.", "A,.", "[A] B.,", "(A]", "[A"] {
        assert!(
            matches!(
                lowering::parse(source),
                Err(Failure::Parse(frontend::failure::Failure::Syntax { .. }))
            ),
            "{source}"
        );
    }
    assert!(syntax("(X, [A] B)").contains("output"));
    assert!(syntax("[([A] B)] C").contains("input"));
    assert!(syntax("[A] ((X, [X] Y), [B] C)").contains("scope"));
    assert!(matches!(
        lowering::parse("[A] (B, C, [B] D)"),
        Err(Failure::Scope { count: 2, .. })
    ));
    assert!(matches!(
        lowering::parse("[Enter] (A.(B,C), [A] D)"),
        Err(Failure::Scope { count: 2, .. })
    ));
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
        assert!(serde_json::from_str::<Program>(source).is_err(), "{source}");
    }
}

#[test]
fn depth() {
    let limit = frontend::parser::DEPTH;
    let nested = |count: usize| format!("{}A{}", "[".repeat(count), "]".repeat(count));
    assert!(lowering::parse(&nested(limit)).is_ok());
    assert!(matches!(
        lowering::parse(&nested(limit + 1)),
        Err(Failure::Parse(frontend::failure::Failure::Depth { .. }))
    ));
    assert!(matches!(
        lowering::parse(&"[A] ".repeat(limit)),
        Err(Failure::Expansion { .. })
    ));
    for count in [limit + 1, 10_000] {
        let Err(Failure::Expansion { span, .. }) = lowering::parse(&"[A] ".repeat(count)) else {
            panic!("expected an expansion diagnostic");
        };
        assert_eq!((span.offset(), span.len()), (0, count * 4 - 1));
    }
    let mixed = format!("{}({})", "(".repeat(limit - 1), "[B] ".repeat(2));
    assert!(matches!(
        lowering::parse(&format!("{mixed}{}", ")".repeat(limit - 1))),
        Err(Failure::Parse(frontend::failure::Failure::Depth { .. }))
    ));
    assert!(lowering::parse(&"[A] B, ".repeat(10_000)).is_ok());
    assert!(matches!(
        lowering::parse(&"[".repeat(limit + 1)),
        Err(Failure::Parse(frontend::failure::Failure::Depth { .. }))
    ));
    assert!(lowering::parse(&format!("{}A{}", "(".repeat(limit), ")".repeat(limit))).is_ok());
}

#[test]
fn distribution() {
    for (compact, expanded) in [
        ("A.(B,C)", "A.B,A.C"),
        ("Pack.(Position.0, Value.2)", "Pack.Position.0,Pack.Value.2"),
        ("A.(B,C).D", "A.B.D,A.C.D"),
        ("(A,B).C.(D,E)", "A.C.D,A.C.E,B.C.D,B.C.E"),
        ("(A,B).(C,D)", "A.C,A.D,B.C,B.D"),
        ("A.(B.(C,D),E)", "A.B.C,A.B.D,A.E"),
        ("A.((),B)", "A,A.B"),
        ("A.(B,B)", "A.B,A.B"),
        ("A.((),())", "A,A"),
    ] {
        assert_eq!(
            lowering::parse(compact).unwrap().initial,
            lowering::parse(expanded).unwrap().initial,
            "{compact}"
        );
        for source in [format!("[{compact}] Result"), format!("[Seed] ({compact})")] {
            let expected = source.replace(compact, expanded);
            assert_eq!(
                lowering::parse(&source).unwrap().rule[0].canonical(),
                lowering::parse(&expected).unwrap().rule[0].canonical(),
                "{source}"
            );
        }
    }
    assert_eq!(
        lowering::parse("[A] (B,C).(D,E)").unwrap().rule[0]
            .output
            .len(),
        4
    );
    let source = lowering::parse("Code.(([A] B),([C] D))").unwrap();
    assert_eq!(source.initial.len(), 2);
    assert!(
        source
            .initial
            .iter()
            .all(|particle| particle.len() == 2 && matches!(particle[1], Value::Rule { .. }))
    );
}

#[test]
fn expansion() {
    let source = format!("A{}", ".(B,C)".repeat(40));
    assert!(matches!(
        lowering::parse(&source),
        Err(Failure::Expansion { .. })
    ));
    assert_eq!(lowering::parse("A.(B,C)").unwrap().initial.len(), 2);
}

#[test]
fn numeral() {
    assert_eq!(
        lowering::parse("0b101.2^0.2^2").unwrap().initial,
        [atom(&["0b101", "2^0", "2^2"])]
    );
    assert_eq!(
        lowering::parse("1.Power.(2,1)").unwrap().initial,
        [atom(&["1", "Power", "2"]), atom(&["1", "Power", "1"])]
    );
}
