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
    assert_eq!(text(&tree, Kind::Rule), ["[A, B] (C.D)"]);
    assert_eq!(text(&tree, Kind::Group), ["(C.D)"]);
    assert_eq!(
        text(&tree, Kind::Term),
        ["A.A", "[A, B] (C.D)", "A", "B", "(C.D)", "C.D"]
    );
    assert_eq!(text(&tree, Kind::Concept), ["A", "A", "A", "B", "C", "D"]);
    let rule = tree
        .node()
        .iter()
        .position(|node| node.kind == Kind::Rule)
        .unwrap();
    let child = tree
        .node()
        .iter()
        .filter(|node| node.parent == Some(rule))
        .map(|node| node.kind)
        .collect::<Vec<_>>();
    assert_eq!(child, [Kind::List, Kind::Term]);
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

#[test]
fn dot() {
    for source in [".A", "A..B", "A.", "A.,B", "(.)"] {
        assert!(
            matches!(parser::parse(source), Err(Failure::Syntax { .. })),
            "{source}"
        );
    }
    for source in ["A.B", "A . B", "A B", "A(B)", "A.(B)", "[A].B"] {
        assert!(parser::parse(source).is_ok(), "{source}");
    }
}

#[test]
fn empty() {
    for source in ["", " \t\r\n", "()", "[]", "[A,,]", ",", "A,,B,", "(,)"] {
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
    assert!(parser::parse(&"[A] ".repeat(limit)).is_ok());
    assert!(matches!(
        parser::parse(&"[A] ".repeat(limit + 1)),
        Err(Failure::Depth { .. })
    ));
    assert!(parser::parse(&"[A] B, ".repeat(limit + 1)).is_ok());
}

#[test]
fn breadth() {
    let source = format!("{}A", "A.".repeat(10_000));
    let tree = parser::parse(&source).unwrap();
    assert_eq!(tree.node().len(), 10_004);
    let source = "A, ".repeat(10_000);
    assert_eq!(parser::parse(&source).unwrap().node().len(), 20_002);
}
