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

fn entry(input: &str, rest: &[&str], output: &str) -> Definition {
    let pattern = |value: &str| vec![atom(&value.split('.').collect::<Vec<_>>())];
    Definition {
        name: String::new(),
        input: pattern(input),
        rest: rest.iter().map(|value| pattern(value)).collect(),
        output: vec![Output {
            particle: atom(&[output]),
            body: None,
        }],
    }
    .canonical()
}

#[test]
fn rest() {
    let (_, lowered) = canonical("[A] [B] C");
    assert_eq!(lowered, [entry("A", &["B"], "C"), entry("B", &["A"], "C")]);
    let (_, lowered) = canonical("[A] [B] [C] D");
    assert_eq!(
        lowered,
        [
            entry("A", &["B", "C"], "D"),
            entry("B", &["A", "C"], "D"),
            entry("C", &["A", "B"], "D"),
        ]
    );
    let (_, lowered) = canonical("[A] [A] B");
    assert_eq!(lowered, [entry("A", &["A"], "B"), entry("A", &["A"], "B")]);
    let (_, lowered) = canonical("[A] B");
    assert_eq!(lowered, [entry("A", &[], "B")]);
    let program = lowering::parse("[A, B] [C] D").unwrap();
    assert_eq!(program.rule[0].rest, [vec![atom(&["C"])]]);
    assert_eq!(program.rule[1].rest, [vec![atom(&["A"]), atom(&["B"])]]);
    let program = lowering::parse("X.([A] [B] C)").unwrap();
    let [particle] = &program.initial[..] else {
        panic!("expected one coherence");
    };
    assert_eq!(particle.len(), 3);
    assert_eq!(rule(&particle[1]).rest, [vec![atom(&["B"])]]);
    let program = lowering::parse("[[A] [B] C] D").unwrap();
    assert_eq!(program.rule[0].input.len(), 1);
    assert_eq!(program.rule[0].input[0].len(), 2);
    let program = lowering::parse("[S] (K, [A] [B] C)").unwrap();
    let body = program.rule[0].output[0].body.as_ref().unwrap();
    assert_eq!(body.len(), 2);
    assert!(body.iter().all(|rule| rule.rest.len() == 1));
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
    let program = lowering::parse("C [A] [B]").unwrap();
    assert_eq!(program.rule[0].name, "[A] [B] C");
    assert_eq!(program.rule[1].name, "[B] [A] C");
    for (source, text) in [
        ("[A] [B] C", "[A] [B] C"),
        ("[A.X, ()] [B] (C, D)", "[A.X, ()] [B] (C, D)"),
        ("[A] (K, [K] L)", "[A] (K, [K] L)"),
        ("[A] ().([K] L)", "[A] ().([K] L)"),
        ("[[K] L] X.([K] L)", "[[K] L] X.([K] L)"),
        ("[A]", "[A]"),
    ] {
        let mut rule = lowering::parse(source).unwrap().rule.remove(0);
        rule.name.clear();
        assert_eq!(frontend::text::definition(&rule), text, "{source}");
    }
}

#[test]
fn edge() {
    let program = lowering::parse("[A], [B] C").unwrap();
    assert!(program.rule[0].output.is_empty() && program.rule[0].rest.is_empty());
    assert_eq!(program.rule[1].output[0].particle, atom(&["C"]));
    assert_ne!(canonical("[A], [B] C"), canonical("[A] [B] C"));
    let program = lowering::parse("[A] [B]").unwrap();
    assert!(program.rule.iter().all(|rule| rule.output.is_empty()));
    let program = lowering::parse("[] [] A").unwrap();
    assert_eq!(program.rule.len(), 2);
    assert!(
        program
            .rule
            .iter()
            .all(|rule| rule.input.is_empty() && rule.rest == [Vec::<Vec<Value>>::new()])
    );
    let program = lowering::parse("[A] [B] (K, [K] C)").unwrap();
    for rule in &program.rule {
        assert_eq!(rule.output[0].particle, atom(&["K"]));
        assert!(rule.output[0].body.is_some());
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
    for (count, fits) in [(1, true), (6, true), (13, true), (14, false), (64, false)] {
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
                    assert!(program.rule.iter().all(|rule| rule.rest.len() == count - 1));
                }
                Err(Failure::Expansion { span, .. }) => {
                    assert!(!fits, "{count} brackets");
                    assert_eq!((span.offset(), span.len()), (0, source.len()));
                }
                Err(other) => panic!("{source}: {other:?}"),
            }
        }
    }
    let program = lowering::parse(&format!("{}, {}", piece(6).join(" "), piece(6).join(" ")));
    assert_eq!(program.unwrap().rule.len(), 12);
}
