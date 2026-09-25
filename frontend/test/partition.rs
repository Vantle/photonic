use frontend::failure::Failure;
use frontend::lowering;
use frontend::source::{Definition, Value};

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

fn syntax(source: &str) -> String {
    match lowering::parse(source) {
        Err(Failure::Syntax { message, .. } | Failure::Lowering { message, .. }) => message,
        other => panic!("expected a syntax failure for {source}, found {other:?}"),
    }
}

#[test]
fn meaning() {
    for (term, spelled) in [
        ("[A] B", "[A] B"),
        ("[A]", "[A]"),
        ("[A] [B]", "[A] B, [B] A"),
        ("[A] [B] C", "[A] B, [A] C, [B] A, [B] C"),
        ("[A] [B] [C]", "[A] B, [A] C, [B] A, [B] C, [C] A, [C] B"),
        ("[A, B] [C] D", "[A, B] C, [A, B] D, [C] (A, B), [C] D"),
        ("[] [A]", "[] A, [A]"),
        ("[()] [A]", "[()] A, [A] ()"),
        ("[A] [B] (C, D)", "[A] B, [A] (C, D), [B] A, [B] (C, D)"),
        (
            "[A] [B] (K, [K] C)",
            "[A] B, [A] (K, [K] C), [B] A, [B] (K, [K] C)",
        ),
        ("[[X] Y] [B]", "[[X] Y] B, [B] ().([X] Y)"),
        ("[A.(B, C)] [D]", "[A.B, A.C] D, [D] (A.B, A.C)"),
        ("[A] [A] B", "[A] A, [A] B, [A] A, [A] B"),
        ("X.([A] [B] C)", "X.([A] B).([A] C).([B] A).([B] C)"),
        ("[[A] [B] C] D", "[([A] B).([A] C).([B] A).([B] C)] D"),
        ("[S] (K, [A] [B] C)", "[S] (K, [A] B, [A] C, [B] A, [B] C)"),
    ] {
        assert_eq!(canonical(term), canonical(spelled), "{term}");
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
    assert_eq!(
        name,
        ["[A] B", "[A] C", "[B] A", "[B] C", "C.D [E]", "[F]  G"]
    );
    for (source, text) in [
        ("[A.X, ()] (C, D)", "[A.X, ()] (C, D)"),
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
    assert_ne!(canonical("[A], [B] C"), canonical("[A] [B] C"));
    let program = lowering::parse("[] []").unwrap();
    assert_eq!(program.rule.len(), 2);
    assert!(
        program
            .rule
            .iter()
            .all(|rule| rule.input.is_empty() && rule.output.is_empty())
    );
    let program = lowering::parse("X.([A] [B])").unwrap();
    assert_eq!(program.initial.len(), 1);
    assert_eq!(program.initial[0].len(), 3);
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
    for (count, fits) in [
        (1, true),
        (6, true),
        (200, true),
        (240, false),
        (10_000, false),
    ] {
        let piece = piece(count);
        let position = if count < 10 {
            (0..=count).collect::<Vec<_>>()
        } else {
            vec![0, count / 2, count]
        };
        for position in position {
            let mut piece = piece.clone();
            let sink = piece.pop().unwrap();
            piece.insert(position, sink);
            let source = piece.join(" ");
            match lowering::parse(&source) {
                Ok(program) => {
                    assert!(fits, "{count} brackets");
                    assert_eq!(program.rule.len(), count * count);
                }
                Err(Failure::Expansion { span, .. }) => {
                    assert!(!fits, "{count} brackets");
                    assert_eq!((span.offset(), span.len()), (0, source.len()));
                }
                Err(other) => panic!("{count} brackets: {other:?}"),
            }
        }
    }
}
