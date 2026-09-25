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
    assert_eq!(text(&tree, Kind::Rule), ["[A, B]"]);
    assert_eq!(text(&tree, Kind::Group), ["(C.D)"]);
    assert_eq!(
        text(&tree, Kind::Term),
        ["A.A", "[A, B] (C.D)", "A", "B", "C.D"]
    );
    assert_eq!(text(&tree, Kind::Concept), ["A", "A", "A", "B", "C", "D"]);
    let rule = tree
        .node()
        .iter()
        .position(|node| node.kind == Kind::Rule)
        .unwrap();
    assert_eq!(child(&tree, rule), [Kind::List]);
    assert_eq!(
        child(&tree, tree.node()[rule].parent.unwrap()),
        [Kind::Rule, Kind::Group]
    );
    let tree = parser::parse("[X] (A.B, C), D.E").unwrap();
    assert_eq!(
        text(&tree, Kind::Term),
        ["[X] (A.B, C)", "X", "A.B", "C", "D.E"]
    );
}

fn child(tree: &Tree<'_>, parent: usize) -> Vec<Kind> {
    tree.node()
        .iter()
        .filter(|node| node.parent == Some(parent))
        .map(|node| node.kind)
        .collect()
}

#[test]
fn ordering() {
    for (source, expected) in [
        (
            "[A] [B] C.D",
            [Kind::Rule, Kind::Rule, Kind::Concept, Kind::Concept],
        ),
        (
            "[A] C.D [B]",
            [Kind::Rule, Kind::Concept, Kind::Concept, Kind::Rule],
        ),
        (
            "C.D [A] [B]",
            [Kind::Concept, Kind::Concept, Kind::Rule, Kind::Rule],
        ),
    ] {
        let tree = parser::parse(source).unwrap();
        let term = tree
            .node()
            .iter()
            .position(|node| node.kind == Kind::Term)
            .unwrap();
        assert_eq!(child(&tree, term), expected, "{source}");
        assert_eq!(text(&tree, Kind::Rule), ["[A]", "[B]"], "{source}");
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
    assert_eq!(text(&tree, Kind::Concept), ["人", "世界", "人", "🌋"]);
    match parser::parse("人]").unwrap_err() {
        Failure::Syntax { span, .. } => {
            assert_eq!(span.offset(), 3);
            assert_eq!(span.len(), 1);
        }
        other => panic!("{other:?}"),
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
        Failure::Syntax { span, .. } => {
            assert_eq!(span.offset(), 4);
            assert_eq!(span.len(), 0);
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
fn dot() {
    for source in [".A", "A..B", "A.", "A.,B", "(.)"] {
        rejected(source);
    }
    for source in [
        "A.B",
        "A . B",
        "A.(B)",
        "([A]).B",
        "X.([A] B)",
        "[A] [B] C",
        "B [A]",
    ] {
        assert!(parser::parse(source).is_ok(), "{source}");
    }
    for (source, offset) in [("X.[A] B", 2), ("[A].B", 3), ("B.[A]", 2), ("[A] B.[C]", 6)] {
        let (found, message) = rejected(source);
        assert_eq!(found, offset, "{source}");
        assert!(message.contains("parentheses"), "{source}");
    }
}

#[test]
fn space() {
    for (source, offset) in [
        ("A B", 2),
        ("A(B)", 1),
        ("(A) (B)", 4),
        ("[A] B C", 6),
        ("C [A] B", 6),
        ("C B [A]", 2),
        ("[A] B [C] D", 10),
        ("B [A] [C] D", 10),
        ("(Kettle [Kettle.Tea] Cup)", 21),
        ("[A] B\n[B] C", 10),
        ("[A B] C", 3),
    ] {
        let (found, message) = rejected(source);
        assert_eq!(found, offset, "{source}");
        assert!(
            message.contains("dot") && message.contains("comma"),
            "{source}"
        );
    }
}

#[test]
fn earliest() {
    for (source, offset) in [
        ("A..B C", 2),
        ("A.B.,\n[A] B C", 4),
        ("(A..B", 3),
        (".A", 0),
        ("A.", 2),
        ("(A.)", 3),
    ] {
        let (found, message) = rejected(source);
        assert_eq!(found, offset, "{source}: {message}");
        assert!(message.contains("dot"), "{source}: {message}");
    }
}

#[test]
fn separator() {
    for character in (0..0x3001).filter_map(char::from_u32) {
        let source = format!("A{character}B");
        let single = parser::parse(&source)
            .is_ok_and(|tree| text(&tree, Kind::Concept) == [source.as_str()]);
        let separating =
            parser::SPACE.contains(&character) || parser::DELIMITER.contains(&character);
        assert_eq!(single, !separating, "{character:?}");
    }
}

#[test]
fn empty() {
    for source in ["", " \t\r\n", "()", "[]", "A,", "[A, B,]", "(A,)"] {
        assert!(parser::parse(source).is_ok(), "{source}");
    }
    for source in [",", ",A", "A,,B", "(,)", "[A,,]"] {
        assert!(rejected(source).1.contains("comma"), "{source}");
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
