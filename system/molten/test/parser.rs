use molten::failure::Failure;
use molten::parser;
use molten::syntax::Kind;

#[test]
fn structure() {
    let source = "A.A, [A, B] (C.D)\r\n";
    let tree = parser::parse(source).unwrap();
    assert_eq!(tree.source(), source);
    assert_eq!(tree.node()[0].span, 0..source.len());
    let context = tree
        .node()
        .iter()
        .position(|node| node.kind == Kind::Context)
        .unwrap();
    assert_eq!(&source[tree.node()[context].span.clone()], "[A, B]");
    let child = tree
        .node()
        .iter()
        .filter(|node| node.parent == Some(context))
        .map(|node| &source[node.span.clone()])
        .collect::<Vec<_>>();
    assert_eq!(child, ["A", ",", " ", "B"]);
    let concept = tree
        .node()
        .iter()
        .filter(|node| node.kind == Kind::Concept)
        .map(|node| &source[node.span.clone()])
        .collect::<Vec<_>>();
    assert_eq!(concept, ["A", "A", "A", "B", "C", "D"]);
}

#[test]
fn unicode() {
    let source = "人.世界 [人] 🌋";
    let tree = parser::parse(source).unwrap();
    let concept = tree
        .node()
        .iter()
        .filter(|node| node.kind == Kind::Concept)
        .map(|node| &source[node.span.clone()])
        .collect::<Vec<_>>();
    assert_eq!(concept, ["人", "世界", "人", "🌋"]);
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
    for source in ["(", "[A", "(A]", "[A)", "A)", "([)]", "A [B] ]"] {
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
fn empty() {
    for source in ["", " \t\r\n", "()", "[]", "[A,,]", ".A", "A..B"] {
        assert!(parser::parse(source).is_ok(), "{source}");
    }
}

#[test]
fn depth() {
    let source = format!("{}人{}", "(".repeat(128), ")".repeat(128));
    assert!(parser::parse(&source).is_ok());
    let source = "[".repeat(129);
    assert!(matches!(
        parser::parse(&source),
        Err(Failure::Depth { limit: 128, .. })
    ));
}

#[test]
fn breadth() {
    let source = "A.".repeat(10_000);
    let tree = parser::parse(&source).unwrap();
    assert_eq!(tree.node().len(), 20_001);
}
