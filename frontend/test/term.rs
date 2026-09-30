use frontend::failure::Failure;
use frontend::lowering;
use frontend::source::Program;

fn canonical(source: &str) -> Program {
    lowering::parse(source)
        .unwrap_or_else(|failure| panic!("{source}: {failure}"))
        .canonical()
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

#[test]
fn meaning() {
    for (term, spelled) in [
        ("[A] B", "[A] B"),
        ("[A]", "[A]"),
        ("[A] [B]", "[A.B]"),
        ("[A] [B] C", "[A.B] C"),
        ("[A] [B] [C]", "[A.B.C]"),
        ("[A, B] [C] D", "[A.C, B.C] D"),
        ("[] [A]", "[]"),
        ("[()] [A]", "[A]"),
        ("[A] [B] (C, D)", "[A.B] (C, D)"),
        ("[A] [B] (K, [K] C)", "[A.B] (K, [K] C)"),
        ("[[X] Y] [B]", "[([X] Y).B]"),
        ("[A.(B, C)] [D]", "[A.B.D, A.C.D]"),
        ("[A] [A] B", "[A.A] B"),
        ("X.([A] [B] C)", "X.([A.B] C)"),
        ("[[A] [B] C] D", "[[A.B] C] D"),
        ("[S] (K, [A] [B] C)", "[S] (K, [A.B] C)"),
        ("C [A] D", "[A] C.D"),
        ("(C, D) [A]", "[A] (C, D)"),
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
    assert_eq!(name, ["[A] [B] C", "C.D [E]", "[F]  G"]);
    for (source, text) in [
        ("[A.X, ()] (C, D)", "[A.X, ()] (C, D)"),
        ("[A] (K, [K] L)", "[A] (K, [K] L)"),
        ("[A] ([K] L)", "[A] ([K] L)"),
        ("[A] (K, M, [K] L)", "[A] (K, M, [K] L)"),
        ("[A] ((K, [K] L), [M] N)", "[A] ([M] N, (K, [K] L))"),
        ("[A] ((), (K, [K] L), [M] N)", "[A] ((), [M] N, (K, [K] L))"),
        ("[A] ().([K] L)", "[A] ().([K] L)"),
        ("[[K] L] X.([K] L)", "[[K] L] X.([K] L)"),
        ("[A] [B] C", "[A.B] C"),
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
    assert_eq!(program.rule.len(), 1);
    assert!(program.rule[0].input.is_empty() && program.rule[0].output.is_empty());
    let program = lowering::parse("X.([A] [B])").unwrap();
    assert_eq!(program.initial.len(), 1);
    assert_eq!(program.initial[0].len(), 2);
    let program = lowering::parse("(K, [K] C) [A], (X, [X] Y)").unwrap();
    assert_eq!(program.rule.len(), 1);
    assert_eq!(program.scope.len(), 1);
}

#[test]
fn limit() {
    let source = |count: usize| {
        (0..count)
            .map(|index| format!("[A{index}, B{index}]"))
            .chain(["Z".to_owned()])
            .collect::<Vec<_>>()
            .join(" ")
    };
    for (count, fits) in [(1, true), (6, true), (12, true), (40, false)] {
        let source = source(count);
        match lowering::parse(&source) {
            Ok(program) => {
                assert!(fits, "{count} brackets");
                assert_eq!(program.rule.len(), 1);
                assert_eq!(program.rule[0].input.len(), 1 << count);
            }
            Err(Failure::Expansion { span, .. }) => {
                assert!(!fits, "{count} brackets");
                assert_eq!((span.offset(), span.len()), (0, source.len()));
            }
            Err(other) => panic!("{count} brackets: {other:?}"),
        }
    }
}
