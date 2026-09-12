use molten::hygiene;
use molten::lowering;
use molten::program::{Instruction, Output, Program, Symbol};
use std::collections::BTreeMap;

fn value(source: &str) -> Vec<Symbol> {
    Program::new(lowering::parse(source).unwrap())
        .initial
        .into_iter()
        .flatten()
        .map(|value| value.close(0))
        .collect()
}

#[test]
fn permutation() {
    let value = value("@([$x.$y] -> $x), @([$a.$b] -> $b), @([$x.$x] -> $x);");
    assert_eq!(value[0], value[1]);
    assert_ne!(value[0], value[2]);
    let Symbol::Rule(rule, _) = &value[0] else {
        panic!("expected a rule")
    };
    assert_eq!(rule.name, "[$x.$y] -> $x");
    assert_eq!(hygiene::instruction(rule), **rule);
}

#[test]
fn generated() {
    let program = Program::new(
        lowering::parse(
            "@([$own] -> Pair(B.$own)); [Make.$outer] -> @([$local] -> Pair($outer.$local));",
        )
        .unwrap(),
    );
    let rule = &program.scope[0].rule[0];
    let name = rule
        .input
        .iter()
        .flatten()
        .find_map(|value| {
            if let Symbol::Variable(name) = value {
                Some(name.clone())
            } else {
                None
            }
        })
        .unwrap();
    let binding = BTreeMap::from([(name, Symbol::Atom(program.atom.get_index_of("B").unwrap()))]);
    let generated = rule.substitute(&binding);
    assert!(generated.ready());
    assert_eq!(
        generated.output[0].particle[0].close(0),
        program.initial[0][0].close(0)
    );
}

#[test]
fn lexical() {
    let value = value(
        "@([$outer] -> { [$inner] -> Pair($outer.$inner); }), @([$a] -> { [$b] -> Pair($a.$b); }), @([$a] -> { [$b] -> Pair($b.$b); });",
    );
    assert_eq!(value[0], value[1]);
    assert_ne!(value[0], value[2]);
    let binding = BTreeMap::from([("$0".to_owned(), Symbol::Atom(999))]);
    assert_eq!(value[0].substitute(&binding), value[0]);
    let renamed = value[0].rename(&mut |frame| frame + 3);
    assert_eq!(renamed.capture(), vec![3]);
}

#[test]
fn existential() {
    let value = value(
        "@([A] unless [Box($query)] -> B), @([A] unless [Box($other)] -> B), @([A] unless [Box($query)] -> $query);",
    );
    assert_eq!(value[0], value[1]);
    let Symbol::Rule(rule, _) = &value[0] else {
        panic!("expected a rule")
    };
    assert!(rule.ready());
    let Symbol::Rule(rule, _) = &value[2] else {
        panic!("expected a rule")
    };
    assert!(!rule.ready());
}

#[test]
fn unresolved() {
    let rule = Instruction {
        name: "free".into(),
        input: vec![vec![Symbol::Variable("x".into())]],
        output: vec![Output {
            particle: vec![Symbol::Variable("$0".into())],
            body: None,
        }],
        negative: None,
    };
    let normalized = hygiene::instruction(&rule);
    assert_ne!(normalized.input[0][0], normalized.output[0].particle[0]);
    assert!(!normalized.ready());
    assert_eq!(hygiene::instruction(&normalized), normalized);
}

#[test]
fn symmetric() {
    let left = (0..12)
        .map(|index| format!("$x{index}"))
        .collect::<Vec<_>>()
        .join(".");
    let right = (0..12)
        .map(|index| format!("$y{}", 11 - index))
        .collect::<Vec<_>>()
        .join(".");
    let value = value(&format!(
        "@([Box({left})] -> Done), @([Box({right})] -> Done);"
    ));
    assert_eq!(value[0], value[1]);
}

#[test]
fn distinguished() {
    let left = (0..12)
        .map(|index| format!("Box(A{index}.$x{index})"))
        .collect::<Vec<_>>()
        .join(".");
    let right = (0..12)
        .rev()
        .map(|index| format!("Box(A{index}.$y{})", 11 - index))
        .collect::<Vec<_>>()
        .join(".");
    let value = value(&format!("@([{left}] -> $x0), @([{right}] -> $y11);"));
    assert_eq!(value[0], value[1]);
}

#[test]
fn ownership() {
    let value = value(
        "@([$x] -> @([$x.$y] -> $x)), @([$a] -> @([$a.$b] -> $a)), @([$a] -> @([$b.$c] -> $b));",
    );
    assert_eq!(value[0], value[1]);
    assert_ne!(value[0], value[2]);
}

#[test]
fn rendering() {
    let program = Program::new(lowering::parse("@([$value] -> Box($value));").unwrap());
    let label = program.label(&program.initial[0][0].close(0));
    assert!(lowering::parse(&format!("{label};")).is_ok());
}
