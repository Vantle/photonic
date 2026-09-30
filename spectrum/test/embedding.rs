use crate::configuration::{Coherence, Configuration, Frame, Occurrence, Opener, Value};
use crate::embedding;
use crate::pattern::{Body, Pattern};
use frontend::source::Definition;

fn body(text: &str) -> Body {
    let Ok(Pattern::Configuration(body)) = Pattern::read(text) else {
        panic!("{text} is a pattern of coherences and scopes");
    };
    body
}

fn rule(text: &str) -> Option<Definition> {
    let program = frontend::lowering::parse(text).expect("the rule lowers");
    program.rule.first().map(Definition::canonical)
}

fn atom(id: usize, name: &str) -> Occurrence {
    Occurrence {
        id,
        value: Value::Atom(name.to_owned()),
    }
}

// The root and, beside it, as many scopes as asked, each holding A and the live rule [A] B.
fn scoped(count: usize) -> Configuration {
    let scope = (1..=count).map(|index| Frame {
        opener: Some(Opener::Program),
        parent: Some(0),
        lexical: Some(0),
        rule: vec![Occurrence {
            id: count + index,
            value: Value::Rule(0),
        }],
        held: Vec::new(),
    });
    Configuration {
        coherence: (1..=count)
            .map(|frame| Coherence {
                frame,
                occurrence: vec![atom(frame, "A")],
            })
            .collect(),
        frame: std::iter::once(Frame {
            opener: None,
            parent: None,
            lexical: None,
            rule: Vec::new(),
            held: Vec::new(),
        })
        .chain(scope)
        .collect(),
        supported: true,
    }
}

// Scopes written alike take frames in one order only, and a scope too many is refused before any
// frame is tried, so many alike scopes are placed without searching their orders.
#[test]
fn alike() {
    let canon = [rule("[A] B")];
    let scope = |count: usize| vec!["(A, [A] B)"; count].join(", ");
    let entry = scoped(64);
    let placed = embedding::assign(&body(&scope(64)), &entry, canon.as_slice())
        .expect("the search stays within its budget")
        .expect("each scope takes a frame");
    assert_eq!(placed.frame, (1..=64).collect::<Vec<_>>());
    assert_eq!(
        embedding::assign(&body(&scope(65)), &entry, canon.as_slice()),
        Ok(None)
    );
    assert_eq!(
        embedding::assign(
            &body(&format!("A, {}", scope(64))),
            &entry,
            canon.as_slice()
        ),
        Ok(None)
    );
    let fewer = embedding::assign(
        &body(&format!("A, {}", scope(63))),
        &entry,
        canon.as_slice(),
    )
    .expect("the search stays within its budget")
    .expect("the coherence part takes the scope no scope part took");
    assert_eq!(fewer.coherence[0].coherence, 63);
}
