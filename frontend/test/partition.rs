use frontend::lowering::{self, Failure};
use frontend::source::{Definition, Output, Value};

fn value(value: &Value) -> Value {
    match value {
        Value::Atom(atom) => Value::Atom(atom.clone()),
        Value::Rule { rule } => Value::Rule {
            rule: Box::new(rule.canonical()),
        },
    }
}

fn canonical(source: &str) -> (Vec<Vec<Value>>, Vec<Definition>) {
    let program = lowering::parse(source).unwrap_or_else(|failure| panic!("{source}: {failure}"));
    let mut initial = program
        .initial
        .iter()
        .map(|particle| {
            let mut particle = particle.iter().map(value).collect::<Vec<_>>();
            particle.sort();
            particle
        })
        .collect::<Vec<_>>();
    initial.sort();
    let mut rule = program
        .rule
        .iter()
        .map(Definition::canonical)
        .collect::<Vec<_>>();
    rule.sort();
    (initial, rule)
}

fn permutation(piece: &[&str]) -> Vec<String> {
    if piece.len() < 2 {
        return vec![piece.join(" ")];
    }
    (0..piece.len())
        .flat_map(|index| {
            let mut rest = piece.to_vec();
            let first = rest.remove(index);
            permutation(&rest)
                .into_iter()
                .map(move |tail| format!("{first} {tail}"))
        })
        .collect()
}

fn atom(value: &[&str]) -> Vec<Value> {
    value
        .iter()
        .map(|value| Value::Atom((*value).to_owned()))
        .collect()
}

fn rule(value: &Value) -> &Definition {
    let Value::Rule { rule } = value else {
        panic!("expected a rule value, found {value:?}");
    };
    rule
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
fn rest() {
    let program = lowering::parse("[A] [B] C").unwrap();
    assert_eq!(program.rule.len(), 2);
    for (rule, (input, other)) in program.rule.iter().zip([("A", "B"), ("B", "A")]) {
        assert_eq!(rule.input, [atom(&[input])]);
        let [
            Output {
                particle,
                body: None,
            },
        ] = &rule.output[..]
        else {
            panic!("expected one coherence, found {:?}", rule.output);
        };
        let [produced] = &particle[..] else {
            panic!("expected one rule value, found {particle:?}");
        };
        let produced = self::rule(produced);
        assert_eq!(produced.input, [atom(&[other])]);
        assert_eq!(produced.output[0].particle, atom(&["C"]));
    }
    for (partition, expanded) in [
        ("[A] B", "[A] B"),
        ("[A] [B] C", "[A] ().([B] C), [B] ().([A] C)"),
        ("[A] [B]", "[A] ().([B]), [B] ().([A])"),
        (
            "[A] [B] [C] D",
            "[A] ().([B] [C] D), [B] ().([A] [C] D), [C] ().([A] [B] D)",
        ),
        (
            "[A] [B] [C] D",
            "[A] ([B] ().([C] D)).([C] ().([B] D)), [B] ([A] ().([C] D)).([C] ().([A] D)), [C] ([A] ().([B] D)).([B] ().([A] D))",
        ),
        ("[A] [B] (C, D)", "[A] ().([B] (C, D)), [B] ().([A] (C, D))"),
        ("[A] [B] ()", "[A] ().([B] ()), [B] ().([A] ())"),
        (
            "[A] [B] (K, [K] C)",
            "[A] ().([B] (K, [K] C)), [B] ().([A] (K, [K] C))",
        ),
        ("[A, B] [C] D", "[A, B] ().([C] D), [C] ().([A, B] D)"),
        ("[] [] A", "[] ().([] A), [] ().([] A)"),
        ("[A] [A] B", "[A] ().([A] B), [A] ().([A] B)"),
        ("X.([A] [B] C)", "X.([A] ().([B] C)).([B] ().([A] C))"),
        ("[[A] [B] C] D", "[([A] ().([B] C)).([B] ().([A] C))] D"),
        (
            "[S] (K, [A] [B] C)",
            "[S] (K, [A] ().([B] C), [B] ().([A] C))",
        ),
    ] {
        assert_eq!(canonical(partition), canonical(expanded), "{partition}");
    }
}

#[test]
fn ordering() {
    for piece in [
        &["[A]", "B"][..],
        &["[A]", "[B]"],
        &["[A]", "[B]", "C"],
        &["[A]", "[B]", "[C]"],
        &["[A]", "[B]", "[C]", "D"],
        &["[A]", "[A]", "B"],
        &["[]", "[()]", "[A, B]", "[[X] Y]", "(D, E)"],
        &["[A.(B, C)]", "[[X] [Y] Z]", "()"],
        &["[A]", "[B]", "(K, [K] L)"],
        &["[A]", "[B]", "X.([K] L).Y"],
        &["[(), ()]", "[A]", "B.C"],
    ] {
        let every = permutation(piece);
        let first = &every[0];
        for (context, expected) in [
            ("{}", first.clone()),
            ("[S] (K, {})", format!("[S] (K, {first})")),
            ("X.({})", format!("X.({first})")),
            ("[{}] Z", format!("[{first}] Z")),
            ("Q, {}, [Q] R", format!("Q, {first}, [Q] R")),
        ] {
            let expected = canonical(&expected);
            for source in &every {
                let source = context.replace("{}", source);
                assert_eq!(canonical(&source), expected, "{source}");
            }
        }
    }
}

#[test]
fn name() {
    let program = lowering::parse("[A] [B] C, C.D [E], [F]  G").unwrap();
    let name = program
        .rule
        .iter()
        .map(|rule| rule.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(name, ["[A] [B] C", "[B] [A] C", "C.D [E]", "[F]  G"]);
    let rest = program.rule[..2]
        .iter()
        .map(|value| rule(&value.output[0].particle[0]).name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(rest, ["[B] C", "[A] C"]);
    let program = lowering::parse("C [A] [B]").unwrap();
    assert_eq!(program.rule[0].name, "[A] [B] C");
    assert_eq!(program.rule[1].name, "[B] [A] C");
}

#[test]
fn edge() {
    let program = lowering::parse("[A], [B] C").unwrap();
    assert!(program.rule[0].output.is_empty());
    assert_eq!(program.rule[1].output[0].particle, atom(&["C"]));
    assert_ne!(canonical("[A], [B] C"), canonical("[A] [B] C"));
    let program = lowering::parse("[A] [B]").unwrap();
    for rule in &program.rule {
        let produced = self::rule(&rule.output[0].particle[0]);
        assert!(produced.output.is_empty());
    }
    let program = lowering::parse("[] [] A").unwrap();
    assert_eq!(program.rule.len(), 2);
    assert!(program.rule.iter().all(|rule| rule.input.is_empty()));
    let program = lowering::parse("X.([A] [B] C)").unwrap();
    assert_eq!(program.initial.len(), 1);
    assert_eq!(program.initial[0].len(), 3);
    let program = lowering::parse("[[A] [B] C] D").unwrap();
    assert_eq!(program.rule.len(), 1);
    assert_eq!(program.rule[0].input.len(), 1);
    assert_eq!(program.rule[0].input[0].len(), 2);
    let program = lowering::parse("[S] (K, [A] [B] C)").unwrap();
    assert_eq!(program.rule[0].output[0].particle, atom(&["K"]));
    assert_eq!(program.rule[0].output[0].body.as_ref().unwrap().len(), 2);
    let program = lowering::parse("[A] [B] (K, [K] C)").unwrap();
    for rule in &program.rule {
        let produced = self::rule(&rule.output[0].particle[0]);
        assert_eq!(produced.output[0].particle, atom(&["K"]));
        assert!(produced.output[0].body.is_some());
    }
    for source in [
        "B [A] C",
        "[A] B C",
        "B C [A]",
        "[A] B [C] D",
        "B [A] [C] D",
        "[A] B [C] D.E",
        "(B) [A] C",
        "[A] (B) C",
    ] {
        let message = syntax(source);
        assert!(
            message.contains("dot") && message.contains("comma"),
            "{source}: {message}"
        );
    }
    for source in ["X.[A] B", "[A].B", "B.[A]", "[A] B.[C]", "[A] [B].C"] {
        assert!(syntax(source).contains("parentheses"), "{source}");
    }
    assert!(syntax("(K, [K] C) [A], (X, [X] Y)").contains("scope"));
}

#[test]
fn limit() {
    let piece = |count: usize| {
        (0..count)
            .map(|index| format!("[A{index}]"))
            .chain(["Z".to_owned()])
            .collect::<Vec<_>>()
    };
    for (count, fits) in [(1, true), (6, true), (7, true), (8, false), (12, false)] {
        let piece = piece(count);
        for position in 0..=count {
            let mut piece = piece.clone();
            let sink = piece.pop().unwrap();
            piece.insert(position, sink);
            let source = piece.join(" ");
            match lowering::parse(&source) {
                Ok(program) => {
                    assert!(fits, "{count} brackets");
                    assert_eq!(program.rule.len(), count);
                }
                Err(Failure::Expansion { span, .. }) => {
                    assert!(!fits, "{count} brackets");
                    assert_eq!((span.offset(), span.len()), (0, source.len()));
                }
                Err(other) => panic!("{source}: {other:?}"),
            }
        }
    }
    let limit = frontend::parser::DEPTH;
    for count in [1, 2, 3] {
        let bracket = "[A] ".repeat(count);
        let sink = |depth: usize| format!("{}B{}", "(".repeat(depth), ")".repeat(depth));
        for depth in [limit - count, limit - count + 1] {
            for source in [
                format!("{bracket}{}", sink(depth)),
                format!("{} {bracket}", sink(depth)),
            ] {
                let result = lowering::parse(&source);
                if depth + count > limit {
                    assert!(
                        matches!(
                            result,
                            Err(Failure::Parse(frontend::failure::Failure::Depth { .. }))
                        ),
                        "{count} brackets beside {depth} groups"
                    );
                } else {
                    assert_eq!(result.unwrap().rule.len(), count);
                }
            }
        }
    }
}
