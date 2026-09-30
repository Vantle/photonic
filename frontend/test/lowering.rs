use frontend::failure::Failure;
use frontend::lowering;
use frontend::source::{Library, Output, Program, Value};
use miette::Diagnostic;

fn atom(value: &[&str]) -> Vec<Value> {
    value
        .iter()
        .map(|value| Value::Atom((*value).to_owned()))
        .collect()
}

fn same(left: &str, right: &str) {
    let canonical = |source: &str| lowering::parse(source).unwrap().canonical();
    assert_eq!(canonical(left), canonical(right), "{left} and {right}");
}

fn particle(output: &Output) -> &[Value] {
    match output {
        Output::Particle(particle) => particle,
        Output::Scope(_) => panic!("expected a particle, found a scope"),
    }
}

fn program(output: &Output) -> &Program {
    match output {
        Output::Scope(program) => program,
        Output::Particle(_) => panic!("expected a scope, found a particle"),
    }
}

fn syntax(source: &str) -> String {
    match lowering::parse(source) {
        Err(Failure::Syntax { message, .. }) => message,
        Err(failure @ Failure::Input { .. }) => failure.to_string(),
        other => panic!("expected a syntax failure for {source}, found {other:?}"),
    }
}

#[test]
fn conjunction() {
    let source = lowering::parse(include_str!("../../program/language/conjunction.wave")).unwrap();
    assert_eq!(source.initial, [atom(&["And", "True", "False", "Extra"])]);
    assert_eq!(source.rule.len(), 3);
    assert_eq!(source.rule[2].input, [atom(&["And", "Boolean", "Boolean"])]);
    let body = program(&source.rule[2].output[0]);
    assert_eq!(body.initial, [Vec::<Value>::new()]);
    assert_eq!(body.rule[1].input, [atom(&["True", "False"])]);
    assert_eq!(particle(&body.rule[1].output[0]), atom(&["False"]));
}

#[test]
fn partition() {
    let source = lowering::parse("A.X, B.Y, [A, B] (C, D), [C, D] E").unwrap();
    assert_eq!(source.initial, [atom(&["A", "X"]), atom(&["B", "Y"])]);
    assert_eq!(source.rule[0].input, [atom(&["A"]), atom(&["B"])]);
    assert_eq!(source.rule[0].output.len(), 2);
    assert_eq!(particle(&source.rule[0].output[0]), atom(&["C"]));
    assert_eq!(particle(&source.rule[0].output[1]), atom(&["D"]));
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
    assert_eq!(particle(&source.rule[0].output[0]), atom(&["A", "B"]));
    assert_eq!(particle(&source.rule[0].output[1]), atom(&["C"]));
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
    assert_eq!(particle(&rule.output[0]), atom(&["B"]));
    let Value::Rule { rule } = &particle(&source.rule[0].output[0])[0] else {
        panic!("expected rule");
    };
    assert_eq!(particle(&rule.output[0]), atom(&["C"]));
    let source = lowering::parse("[[A,B]] ([B])").unwrap();
    let Value::Rule { rule } = &source.rule[0].input[0][0] else {
        panic!("expected rule");
    };
    assert_eq!(rule.input, [atom(&["A"]), atom(&["B"])]);
    assert!(rule.output.is_empty());
    assert_eq!(
        program(&source.rule[0].output[0]).rule[0].input,
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
    let outer = program(&source.rule[0].output[0]);
    assert_eq!(outer.initial, [atom(&["Make"])]);
    let Value::Rule { rule } = &particle(&outer.rule[0].output[0])[0] else {
        panic!("expected rule");
    };
    assert_eq!(program(&rule.output[0]).rule[0].input, [atom(&["Payload"])]);
    let source = lowering::parse("[A] (B, (X, [X] Y))").unwrap();
    assert_eq!(particle(&source.rule[0].output[0]), atom(&["B"]));
    let inner = program(&source.rule[0].output[1]);
    assert_eq!(inner.initial, [atom(&["X"])]);
    assert_eq!(inner.rule.len(), 1);
    let source = lowering::parse("[A] ([B] C, [C] D,)").unwrap();
    let body = program(&source.rule[0].output[0]);
    assert_eq!(body.initial, [Vec::<Value>::new()]);
    assert_eq!(body.rule.len(), 2);
    let source = lowering::parse("[A] X.([X] Y)").unwrap();
    assert!(matches!(
        particle(&source.rule[0].output[0]),
        [Value::Atom(_), Value::Rule { .. }]
    ));
    let source = lowering::parse("[A] (B, C, [B] D)").unwrap();
    let body = program(&source.rule[0].output[0]);
    assert_eq!(body.initial, [atom(&["B"]), atom(&["C"])]);
    assert_eq!(body.rule.len(), 1);
    same("[Enter] (A.(B, C), [A] D)", "[Enter] (A.B, A.C, [A] D)");
    let source = lowering::parse("[A] ((), B, [B] C)").unwrap();
    assert_eq!(
        program(&source.rule[0].output[0]).initial,
        [Vec::new(), atom(&["B"])]
    );
    let source = lowering::parse("[A] ((X, [X] Y), [B] C)").unwrap();
    let outer = program(&source.rule[0].output[0]);
    assert!(outer.initial.is_empty());
    assert_eq!(outer.scope[0].initial, [atom(&["X"])]);
    let source = lowering::parse("[A] ((), (X, [X] Y), [B] C)").unwrap();
    assert_eq!(
        program(&source.rule[0].output[0]).initial,
        [Vec::<Value>::new()]
    );
    let source = lowering::parse("Z, (X, [X] Y), ([B] C)").unwrap();
    assert_eq!(source.initial, [atom(&["Z"])]);
    assert_eq!(source.scope[0].initial, [atom(&["X"])]);
    assert_eq!(source.scope[1].initial, [Vec::<Value>::new()]);
    assert!(source.rule.is_empty());
}

#[test]
fn empty() {
    let source = lowering::parse("(), [()] A, [A] (), [A], [] A, [(), ()] B").unwrap();
    assert_eq!(source.initial, [Vec::<Value>::new()]);
    assert_eq!(source.rule[0].input, [Vec::<Value>::new()]);
    assert_eq!(source.rule[1].output.len(), 1);
    assert!(particle(&source.rule[1].output[0]).is_empty());
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
    assert_eq!(particle(&source.rule[0].output[0]), atom(&["Box", "B"]));
    for value in [
        r#"{"variable":"x"}"#,
        r#"{"structure":"Box","particle":["A"]}"#,
    ] {
        assert!(Program::read(&format!(r#"{{"initial":[[{value}]]}}"#)).is_err());
    }
}

#[test]
fn whitespace() {
    same("\u{FEFF}A, [A] B", "A, [A] B");
    same("A,\u{FEFF}[A]\u{FEFF}B\u{FEFF}", "A, [A] B");
    same("A,\u{00A0}[A]\u{3000}B\u{2028}", "A, [A] B");
}

#[test]
fn encoding() {
    assert_eq!(
        frontend::encoding::decode("plain.wave", "A, [A] B".into()).unwrap(),
        "A, [A] B"
    );
    let failure = frontend::encoding::decode("wide.wave", vec![0xFF, 0xFE, b'A', 0]).unwrap_err();
    assert_eq!(
        failure.to_string(),
        "wide.wave is not UTF-8 text; save it as UTF-8"
    );
    assert_eq!(
        failure.code().map(|code| code.to_string()).as_deref(),
        Some("photonic::encoding")
    );
}

#[test]
fn point() {
    let composed = lowering::parse("\u{E9}").unwrap();
    let decomposed = lowering::parse("e\u{301}").unwrap();
    assert_eq!(composed.initial, [atom(&["\u{E9}"])]);
    assert_eq!(decomposed.initial, [atom(&["e\u{301}"])]);
    assert_ne!(composed.canonical(), decomposed.canonical());
}

#[test]
fn unicode() {
    let source = lowering::parse("人.世界, [人] 🌋").unwrap();
    assert_eq!(source.initial, [atom(&["人", "世界"])]);
    assert_eq!(particle(&source.rule[0].output[0]), atom(&["🌋"]));
    let Failure::Syntax { span, .. } = lowering::parse("人]").unwrap_err() else {
        panic!("expected diagnostic");
    };
    assert_eq!(span.offset(), 3);
    assert_eq!(span.len(), 1);
}

#[test]
fn malformed() {
    for source in ["A..B", ".A", "[A] B.", "A,.", "[A] B.,", "(A]", "[A"] {
        assert!(
            matches!(lowering::parse(source), Err(Failure::Syntax { .. })),
            "{source}"
        );
    }
    assert!(syntax("[([A] B)] C").contains("input"));
}

#[test]
fn code() {
    for (source, code) in [
        ("A B", "photonic::syntax"),
        (&"[".repeat(frontend::parser::DEPTH + 1), "photonic::depth"),
        ("[([A] B)] C", "photonic::input"),
        (&"[A] ".repeat(10_000), "photonic::expansion"),
    ] {
        let failure = lowering::parse(source).unwrap_err();
        assert_eq!(
            failure.code().map(|code| code.to_string()).as_deref(),
            Some(code),
            "{failure}"
        );
    }
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
        r#"{"rule":[{"input":[["A"]],"output":[["B"]],"negative":[["C"]]}]}"#,
        r#"{"initial":[[{"rule":{"input":[["A"]],"output":[]},"negative":[["C"]]}]]}"#,
        r#"{"initial":[[{"rule":{"input":[["A"]],"output":[],"negative":[["C"]]}}]]}"#,
    ] {
        assert!(serde_json::from_str::<Program>(source).is_err(), "{source}");
    }
    for (source, message) in [
        (r#"{"scope":[{"initial":[["A"]]}]}"#, "lists a rule"),
        (
            r#"{"scope":[{"rule":[{"input":[["A"]],"output":[]}]}]}"#,
            "holds a coherence",
        ),
        (
            r#"{"rule":[{"input":[["A"]],"output":[{"initial":[],"rule":[{"input":[],"output":[]}]}]}]}"#,
            "holds a coherence",
        ),
        (
            r#"{"initial":[[{"rule":{"input":[],"output":[{"initial":[["B"]],"rule":[]}]}}]]}"#,
            "lists a rule",
        ),
    ] {
        let failure = Program::read(source).unwrap_err().to_string();
        assert!(failure.contains(message), "{source}: {failure}");
    }
    for source in [
        r#"{"scope":[{"initial":[[]],"rule":[{"input":[["A"]],"output":[]}]}]}"#,
        r#"{"scope":[{"rule":[{"input":[["A"]],"output":[]}],"scope":[{"initial":[["B"]],"rule":[{"input":[["B"]],"output":[]}]}]}]}"#,
    ] {
        let program = Program::read(source).unwrap();
        assert_eq!(
            lowering::parse(&frontend::text::scope(&program.scope[0]))
                .unwrap()
                .scope[0]
                .canonical(),
            program.scope[0].canonical(),
            "{source}"
        );
    }
}

#[test]
fn depth() {
    let limit = frontend::parser::DEPTH;
    let nested = |count: usize| format!("{}A{}", "[".repeat(count), "]".repeat(count));
    assert!(lowering::parse(&nested(limit)).is_ok());
    assert!(matches!(
        lowering::parse(&nested(limit + 1)),
        Err(Failure::Depth { .. })
    ));
    for count in [limit, limit + 1] {
        assert_eq!(
            lowering::parse(&"[A] ".repeat(count)).unwrap().rule.len(),
            count * (count - 1)
        );
    }
    let Err(Failure::Expansion { span, .. }) = lowering::parse(&"[A] ".repeat(10_000)) else {
        panic!("expected an expansion diagnostic");
    };
    assert_eq!((span.offset(), span.len()), (0, 10_000 * 4 - 1));
    let mixed = format!("{}({})", "(".repeat(limit - 1), "[B] ".repeat(2));
    assert!(matches!(
        lowering::parse(&format!("{mixed}{}", ")".repeat(limit - 1))),
        Err(Failure::Depth { .. })
    ));
    assert!(lowering::parse(&"[A] B, ".repeat(10_000)).is_ok());
    let atom = "A".repeat(20_000);
    for count in [8, 100, limit] {
        let nested = format!("{}{atom}{}", "[".repeat(count), "]".repeat(count));
        assert!(lowering::parse(&nested).is_ok(), "{count}");
    }
    assert!(matches!(
        lowering::parse(&"[".repeat(limit + 1)),
        Err(Failure::Depth { .. })
    ));
    assert!(lowering::parse(&format!("{}A{}", "(".repeat(limit), ")".repeat(limit))).is_ok());
}

#[test]
fn nesting() {
    let inner = (0..3000)
        .map(|index| format!("[R{index}] S{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    for count in [1, 40, frontend::parser::DEPTH - 1] {
        let mut source = format!("[Enter0] (Seed, {inner})");
        for level in 1..count {
            source = format!("[Enter{level}] (Seed, {source})");
        }
        let program = lowering::parse(&format!("Enter{}, {source}", count - 1)).unwrap();
        let written = serde_json::to_string(&program).unwrap();
        assert!(written.len() < 4 * source.len(), "{count}");
    }
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
    let unit = vec!["Unit"; 800].join(".");
    for (left, right) in [
        (format!("(A, B).{unit}"), format!("{unit}.(A, B)")),
        (
            format!("(A, B).{unit}.(C, D)"),
            format!("{unit}.(A, B).(C, D)"),
        ),
    ] {
        same(&left, &right);
    }
    let pair = vec!["(A, B)"; 20].join(".");
    for source in [
        format!("{pair}.{unit}"),
        format!("{unit}.{pair}"),
        format!("X, [Y] {pair}.{unit}"),
    ] {
        let Err(Failure::Expansion { span, .. }) = lowering::parse(&source) else {
            panic!("expected an expansion diagnostic for {source}");
        };
        let join = source.len() - pair.len() - unit.len() - 1;
        assert_eq!((span.offset(), span.len()), (join, source.len() - join));
    }
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

#[test]
fn read() {
    let mut source = String::from("[X] C");
    for _ in 1..frontend::parser::DEPTH {
        source = format!("[A] ().({source})");
    }
    let program = lowering::parse(&source).unwrap();
    let text = serde_json::to_string(&program).unwrap();
    let read = Program::read(&text).unwrap();
    assert_eq!(serde_json::to_string(&read).unwrap(), text);
    assert!(Program::read(&"[".repeat(100_000)).is_err());
    assert!(Program::read(&"{\"rule\":[".repeat(100_000)).is_err());
    assert!(Program::read(r#"{"initial": [["[{\""]]}"#).is_err());
}

fn refused(text: &str) -> (String, usize) {
    match Program::read(text) {
        Err(Failure::Json { message, span }) => (message, span.offset()),
        other => panic!("expected a refusal of {text}, found {other:?}"),
    }
}

#[test]
fn format() {
    for (text, message, offset) in [
        (
            r#"{"rule":[{"input":[["A"]],"output":[[["X"]],[{"input":[["B"]],"output":[]}],[]]}]}"#,
            "invalid type: sequence, expected a value: an atom, written as a string, or a rule, written as {\"rule\": …}",
            37,
        ),
        (
            "[]",
            "invalid type: sequence, expected a program, an object with initial, rule and scope",
            1,
        ),
        (
            r#"[[["A"]],[["",[["A"]],[["B"]]]]]"#,
            "invalid type: sequence, expected a program, an object with initial, rule and scope",
            0,
        ),
        (
            r#"{"rule":[["[Z] W", [["A"]], [["B"]]]], "initial":[["A"]]}"#,
            "invalid type: sequence, expected a rule, an object with input and output",
            9,
        ),
        (
            r#"{"initial":[["A", [["", [["X"]], [["Y"]]]]]]}"#,
            "invalid type: sequence, expected a value: an atom, written as a string, or a rule, written as {\"rule\": …}",
            18,
        ),
        (
            r#"{"rule":[{"input":[["A"]],"output":["B"]}]}"#,
            "invalid type: string \"B\", expected an output: a particle, written as an array, or a scope, written as an object",
            38,
        ),
        (
            r#"{"rule":[{"name":"[B] C","input":[["A"]],"output":[["X"]]}]}"#,
            "unknown field `name`, expected `input` or `output`",
            15,
        ),
        (
            r#"{"rule":[{"input":[["A"]]}]}"#,
            "missing field `output`",
            25,
        ),
        (
            r#"{"initial":[["A"]],"initial":[]}"#,
            "duplicate field `initial`",
            27,
        ),
        (
            r#"{"initial":[[{"rule":{"input":[],"output":[]},"name":"A"}]]}"#,
            "unknown field `name`, there are no fields",
            51,
        ),
        (r#"{"initial":[["A"]]} B"#, "trailing characters", 20),
        (
            "{\n  \"initial\": [[\"A\"]],\n  \"rule\": [\n",
            "EOF while parsing a list",
            36,
        ),
    ] {
        assert_eq!(refused(text), (message.to_owned(), offset), "{text}");
    }
    let program = Program::read("\u{FEFF}{\"initial\":[[\"A\"]]}").unwrap();
    assert_eq!(program.initial, [atom(&["A"])]);
    assert_eq!(
        Program::read("\u{FEFF}{\"initial\":[[\"A\"]],}")
            .unwrap_err()
            .to_string(),
        "invalid Photonic program: trailing comma"
    );
    let failure = Program::read("[]").unwrap_err();
    assert_eq!(
        failure.code().map(|code| code.to_string()).as_deref(),
        Some("photonic::json")
    );
}

#[test]
fn spelling() {
    for (atom, message) in [
        ("A.B", "'.' separates atoms, so an atom cannot hold it"),
        ("", "an atom holds at least one character"),
        (
            "A B",
            "U+0020 is a space, which separates atoms, so an atom cannot hold it",
        ),
        ("(x)", "'(' separates atoms, so an atom cannot hold it"),
        (
            "\u{FEFF}A",
            "U+FEFF is a space, which separates atoms, so an atom cannot hold it",
        ),
        ("A\u{200B}B", "U+200B cannot appear in an atom"),
        ("X\u{1B}c", "U+001B cannot appear in an atom"),
    ] {
        let written = serde_json::to_string(atom).unwrap();
        for text in [
            format!(r#"{{"initial":[[{written}]]}}"#),
            format!(r#"{{"rule":[{{"input":[[{written}]],"output":[]}}]}}"#),
            format!(r#"{{"rule":[{{"input":[],"output":[["A",{written}]]}}]}}"#),
        ] {
            let (found, offset) = refused(&text);
            assert_eq!(found, message, "{text}");
            assert_eq!(
                offset,
                text.find(&written).unwrap() + written.len() - 1,
                "{text}"
            );
        }
    }
    assert!(Program::read(r#"{"initial":[["\u00e9", "e\u0301", "👨‍👩‍👧"]]}"#).is_ok());
}

#[test]
fn library() {
    let library = lowering::library("[A] [B], [C] D,\n[[E] F] ().([G] H)").unwrap();
    assert_eq!(library.rule.len(), 4);
    for (source, offset, length) in [
        ("A, [A] B", 0, 1),
        ("[A] B, (X, [X] Y)", 7, 10),
        ("[A] B,\n  Seed.(C, D), (E)", 9, 11),
        ("[A] B, ()", 7, 2),
    ] {
        let Err(Failure::Library { span }) = lowering::library(source) else {
            panic!("expected {source} to be refused as a library");
        };
        assert_eq!((span.offset(), span.len()), (offset, length), "{source}");
    }
    assert_eq!(
        lowering::library("X").unwrap_err().to_string(),
        "this library lists a coherence or scope; a library holds only rules"
    );
    assert!(matches!(
        lowering::library("[A] B C"),
        Err(Failure::Syntax { .. })
    ));
    let rule = r#"{"input":[["A"]],"output":[["B"]]}"#;
    let library =
        Library::read(&format!(r#"{{"initial":[],"rule":[{rule}],"scope":[]}}"#)).unwrap();
    assert_eq!(library.rule, lowering::parse("[A] B").unwrap().rule);
    for (text, offset) in [
        (format!(r#"{{"rule":[{rule}],"initial":[[],["A"]]}}"#), 57),
        (
            format!(
                r#"{{"rule":[{rule}],"scope":[{{"initial":[[]],"rule":[{rule}]}}],"initial":[["B"]]}}"#
            ),
            54,
        ),
    ] {
        let Err(Failure::Library { span }) = Library::read(&text) else {
            panic!("expected {text} to be refused as a library");
        };
        assert_eq!(span.offset(), offset, "{text}");
    }
    assert!(matches!(
        Library::read(r#"{"rule":[{"input":[["A"]]}],"initial":[["B"]]}"#),
        Err(Failure::Json { .. })
    ));
    let mut program = Program::default();
    program.declare(lowering::library("[A] B").unwrap());
    assert_eq!(program, lowering::parse("[A] B").unwrap());
}

#[test]
fn level() {
    let nested = |count: usize| {
        let mut value = r#""A""#.to_owned();
        for _ in 0..count {
            value = format!(r#"{{"rule":{{"input":[],"output":[[{value}]]}}}}"#);
        }
        format!(r#"{{"initial":[[{value}]]}}"#)
    };
    let limit = frontend::parser::DEPTH;
    assert!(Program::read(&nested(limit - 1)).is_ok());
    let (message, _) = refused(&nested(limit));
    assert_eq!(
        message,
        "this nests deeper than the 128 levels text can write"
    );
    let text = format!("{}A{}", "().([] ".repeat(limit - 1), ")".repeat(limit - 1));
    let lowered = lowering::parse(&text).unwrap();
    assert_eq!(
        Program::read(&serde_json::to_string(&lowered).unwrap()).unwrap(),
        lowered
    );
    let scope = |count: usize| {
        let mut value = r#"{"initial":[[]],"rule":[{"input":[],"output":[]}]}"#.to_owned();
        for _ in 1..count {
            value = format!(r#"{{"rule":[{{"input":[],"output":[]}}],"scope":[{value}]}}"#);
        }
        format!(r#"{{"scope":[{value}]}}"#)
    };
    assert!(Program::read(&scope(limit - 1)).is_ok());
    assert!(refused(&scope(limit + 1)).0.contains("128 levels"));
    assert!(Program::read(&"{\"scope\":[".repeat(1_000_000)).is_err());
}
