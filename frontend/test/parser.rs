use frontend::character;
use frontend::failure::Failure;
use frontend::parser;
use frontend::syntax::{Kind, Tree};

fn text<'source>(tree: &Tree<'source>, kind: Kind) -> Vec<&'source str> {
    tree.node()
        .iter()
        .filter(|node| node.kind == kind)
        .map(|node| tree.source()[node.span.clone()].trim_end())
        .collect()
}

#[test]
fn structure() {
    let source = "A.A, [A, B] (C.D)\r\n";
    let tree = parser::parse(source).unwrap();
    assert_eq!(tree.source(), source);
    assert_eq!(tree.node()[0].kind, Kind::Module);
    assert_eq!(tree.node()[0].span, 0..source.len());
    assert_eq!(text(&tree, Kind::Bracket), ["[A, B]"]);
    assert_eq!(text(&tree, Kind::Group), ["(C.D)"]);
    assert_eq!(
        text(&tree, Kind::Term),
        ["A.A", "[A, B] (C.D)", "A", "B", "C.D"]
    );
    assert_eq!(text(&tree, Kind::Atom), ["A", "A", "A", "B", "C", "D"]);
    let rule = tree
        .node()
        .iter()
        .position(|node| node.kind == Kind::Bracket)
        .unwrap();
    assert_eq!(child(&tree, rule), [Kind::List]);
    let parent = (0..tree.node().len())
        .find(|&index| tree.child(index).any(|child| child == rule))
        .unwrap();
    assert_eq!(child(&tree, parent), [Kind::Bracket, Kind::Group]);
    assert_eq!(child(&tree, 0), [Kind::List]);
    assert_eq!(tree.child(1).count(), 2);
    let tree = parser::parse("[X] (A.B, C), D.E").unwrap();
    assert_eq!(
        text(&tree, Kind::Term),
        ["[X] (A.B, C)", "X", "A.B", "C", "D.E"]
    );
}

fn child(tree: &Tree<'_>, parent: usize) -> Vec<Kind> {
    tree.child(parent)
        .map(|index| tree.node()[index].kind)
        .collect()
}

#[test]
fn ordering() {
    for (source, expected) in [
        (
            "[A] [B] C.D",
            [Kind::Bracket, Kind::Bracket, Kind::Atom, Kind::Atom],
        ),
        (
            "[A] C.D [B]",
            [Kind::Bracket, Kind::Atom, Kind::Atom, Kind::Bracket],
        ),
        (
            "C.D [A] [B]",
            [Kind::Atom, Kind::Atom, Kind::Bracket, Kind::Bracket],
        ),
    ] {
        let tree = parser::parse(source).unwrap();
        let term = tree
            .node()
            .iter()
            .position(|node| node.kind == Kind::Term)
            .unwrap();
        assert_eq!(child(&tree, term), expected, "{source}");
        assert_eq!(text(&tree, Kind::Bracket), ["[A]", "[B]"], "{source}");
    }
    for source in [
        "[A] [B]",
        "[A]",
        "(C) [A]",
        "[A] (C, [C] D) [B]",
        "[[A] [B] C] D",
    ] {
        assert!(parser::parse(source).is_ok(), "{source}");
    }
}

#[test]
fn unicode() {
    let source = "人.世界, [人] 🌋";
    let tree = parser::parse(source).unwrap();
    assert_eq!(text(&tree, Kind::Atom), ["人", "世界", "人", "🌋"]);
    match parser::parse("人]").unwrap_err() {
        Failure::Syntax { span, .. } => {
            assert_eq!(span.offset(), 3);
            assert_eq!(span.len(), 1);
        }
        other => panic!("{other:?}"),
    }
}

fn rejected(source: &str) -> (usize, String) {
    match parser::parse(source) {
        Err(Failure::Syntax { message, span }) => (span.offset(), message),
        other => panic!("expected a syntax failure for {source}, found {other:?}"),
    }
}

#[test]
fn delimiter() {
    for source in ["(", "[A", "(A]", "[A)", "A)", "([)]", "A, [B] ]"] {
        assert!(
            matches!(parser::parse(source), Err(Failure::Syntax { .. })),
            "{source}"
        );
    }
    match parser::parse("[人").unwrap_err() {
        Failure::Syntax { span, message } => {
            assert_eq!((span.offset(), span.len()), (0, 1));
            assert_eq!(message, "this [ is never closed");
        }
        other => panic!("{other:?}"),
    }
    for (source, offset, message) in [
        (
            "Start,\n[Start] (Kettle,\n    [Kettle] Tea,\n\n[Tea] Cup,\n[Cup] Done,\n",
            15,
            "this ( is never closed",
        ),
        ("(A, (B", 4, "this ( is never closed"),
        ("[A] (B, [C", 8, "this [ is never closed"),
        ("(A]", 2, "this ] does not close the ( at 1:1"),
        ("([)]", 2, "this ) does not close the [ at 1:2"),
        ("A,\n  [B, (C]", 11, "this ] does not close the ( at 2:7"),
        ("人, (人]", 9, "this ] does not close the ( at 1:4"),
        ("A)", 1, "this ) closes nothing that is open"),
        ("A, [B] ]", 7, "this ] closes nothing that is open"),
        ("(A.", 0, "this ( is never closed"),
    ] {
        assert_eq!(rejected(source), (offset, message.to_owned()), "{source:?}");
    }
}

#[test]
fn masking() {
    let limit = parser::DEPTH;
    let deep = format!("{}A{}", "[".repeat(limit + 1), "]".repeat(limit + 1));
    assert_eq!(
        rejected(&format!("A B),\n{deep}")),
        (3, "this ) closes nothing that is open".to_owned())
    );
    assert_eq!(
        rejected(&format!("A\u{200B}B,\n{deep}")),
        (1, "U+200B cannot appear in an atom".to_owned())
    );
    assert_eq!(
        rejected(&format!("({deep}")),
        (0, "this ( is never closed".to_owned())
    );
    for (source, offset) in [
        (deep.clone(), limit),
        (format!("{deep} B C"), limit),
        ("[".repeat(limit + 1), limit),
        (format!("A, {}", "(".repeat(limit + 1)), limit + 3),
    ] {
        let Err(Failure::Depth { span, .. }) = parser::parse(&source) else {
            panic!("expected a depth failure for {source}");
        };
        assert_eq!(span.offset(), offset, "{source}");
    }
}

#[test]
fn general() {
    for source in [
        "A B",
        "A(B, C)",
        "(A) (B)",
        "[A] B C",
        "C B [A]",
        "[A] B\n[B] C",
        "X.[A] B",
        "[A].B",
        "[A] [B]",
        ".A",
        "A.",
        "A..B",
        "(.)",
        ".",
        ",",
        ",A",
        "A,,B",
        "(,)",
        "[A,,]",
    ] {
        assert!(parser::parse(source).is_ok(), "{source}");
    }
}

#[test]
fn separator() {
    for character in (0..=0xFFFF)
        .chain([0x1_F600, 0xE_0001, 0x10_FFFF])
        .filter_map(char::from_u32)
    {
        let source = format!("A{character}B");
        if character::refused(character) {
            assert_eq!(
                rejected(&source),
                (
                    1,
                    format!("{} cannot appear in an atom", character::point(character))
                ),
                "{character:?}"
            );
            continue;
        }
        if matches!(character, '(' | ')' | '[' | ']') {
            assert!(
                matches!(parser::parse(&source), Err(Failure::Syntax { .. })),
                "{character:?}"
            );
            continue;
        }
        let tree = parser::parse(&source).unwrap();
        let expected = if character::separator(character) {
            vec!["A", "B"]
        } else {
            vec![source.as_str()]
        };
        assert_eq!(text(&tree, Kind::Atom), expected, "{character:?}");
    }
}

#[test]
fn whitespace() {
    for space in [
        '\u{FEFF}', '\u{0085}', '\u{200E}', '\u{200F}', '\u{2028}', '\u{2029}', '\u{000B}',
    ] {
        assert!(character::space(space), "{space:?}");
        let source = format!("{space}A,{space}[A]{space}B{space}");
        let tree = parser::parse(&source).unwrap();
        assert_eq!(text(&tree, Kind::Atom), ["A", "A", "B"], "{space:?}");
        let joined = format!("A{space}B");
        let tree = parser::parse(&joined).unwrap();
        assert_eq!(text(&tree, Kind::Term), [joined.as_str()], "{space:?}");
    }
    for joiner in [
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
        "\u{0645}\u{06CC}\u{200C}\u{062E}",
        "\u{1F1F3}\u{1F1F1}",
        "\u{2764}\u{FE0F}",
    ] {
        let tree = parser::parse(joiner).unwrap();
        assert_eq!(text(&tree, Kind::Atom), [joiner]);
    }
}

#[test]
fn refused() {
    for (source, offset, point) in [
        ("A\u{200B}B", 1, "U+200B"),
        ("X\u{1B}c, [X] Y", 1, "U+001B"),
        ("A\u{0}", 1, "U+0000"),
        ("\u{202E}A", 0, "U+202E"),
        ("[A] B\u{2066}", 5, "U+2066"),
        ("A, \u{2060}", 3, "U+2060"),
        ("\u{061C}", 0, "U+061C"),
        ("A.\u{7F}", 2, "U+007F"),
        ("A\u{9B}", 1, "U+009B"),
        ("A\u{A0}B", 1, "U+00A0"),
        ("A,\u{3000}B", 2, "U+3000"),
        ("[A] \u{202F}", 4, "U+202F"),
    ] {
        assert_eq!(
            rejected(source),
            (offset, format!("{point} cannot appear in an atom")),
            "{source:?}"
        );
    }
}

#[test]
fn empty() {
    for source in [
        "",
        " \t\r\n\u{85}\u{2028}",
        "()",
        "[]",
        "A,",
        "[A, B,]",
        "(A,)",
    ] {
        assert!(parser::parse(source).is_ok(), "{source}");
    }
}

#[test]
fn depth() {
    let limit = parser::DEPTH;
    let source = format!("{}人{}", "(".repeat(limit), ")".repeat(limit));
    assert!(parser::parse(&source).is_ok());
    assert!(matches!(
        parser::parse(&"[".repeat(limit + 1)),
        Err(Failure::Depth { limit: 128, .. })
    ));
    for opening in ["[", "("] {
        let closing = if opening == "[" { "]" } else { ")" };
        let nested =
            |count: usize| format!("{}A{} [B]", opening.repeat(count), closing.repeat(count));
        assert!(parser::parse(&nested(limit)).is_ok(), "{opening}");
        let Err(Failure::Depth { span, .. }) = parser::parse(&nested(limit + 1)) else {
            panic!("expected a depth failure for {opening}");
        };
        assert_eq!(span.offset(), limit, "{opening}");
    }
    for count in [limit, limit + 1, 10_000] {
        for source in ["[A] ".repeat(count), format!("B {}", "[A] ".repeat(count))] {
            assert!(parser::parse(&source).is_ok(), "{count} brackets");
        }
    }
    assert!(parser::parse(&"[A] B, ".repeat(limit + 1)).is_ok());
    assert!(parser::parse(&format!("{}A{}", "[".repeat(limit), "]".repeat(limit))).is_ok());
    assert!(
        parser::parse(&format!(
            "[{}A{}] B",
            "(".repeat(limit - 1),
            ")".repeat(limit - 1)
        ))
        .is_ok()
    );
    assert!(matches!(
        parser::parse(&format!("[{}A{}] B", "(".repeat(limit), ")".repeat(limit))),
        Err(Failure::Depth { .. })
    ));
}

#[test]
fn breadth() {
    let source = format!("{}A", "A.".repeat(10_000));
    let tree = parser::parse(&source).unwrap();
    assert_eq!(tree.node().len(), 10_004);
    let source = "A, ".repeat(10_000);
    assert_eq!(parser::parse(&source).unwrap().node().len(), 20_002);
}
