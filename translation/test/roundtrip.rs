use super::support::{configuration, flat, nested, vocabulary};
use crate::vocabulary::Vocabulary;
use crate::{emit, lift, text};
use random::Generator;

#[test]
fn source() {
    let mut generator = Generator::new(11);
    for index in 0..800 {
        let program = if index % 2 == 0 {
            flat(&mut generator)
        } else {
            nested(&mut generator)
        };
        let configuration = configuration(&mut generator);
        let mut known = vocabulary();
        let emitted = emit::program(&program, &configuration, &known);
        let (lifted, initial) = lift::program(&emitted, &mut known).unwrap();
        assert_eq!(lifted, program);
        assert_eq!(initial, configuration);
    }
}

#[test]
fn written() {
    let mut generator = Generator::new(12);
    for index in 0..800 {
        let program = if index % 2 == 0 {
            flat(&mut generator)
        } else {
            nested(&mut generator)
        };
        let configuration = configuration(&mut generator);
        let known = vocabulary();
        let written = format!(
            "{},\n{}",
            text::configuration(&configuration, &known),
            text::program(&program, &known)
        );
        let parsed = frontend::lowering::parse(&written)
            .unwrap_or_else(|failure| panic!("{failure}: {written}"));
        let mut lifted = vocabulary();
        let (rule, initial) = lift::program(&parsed, &mut lifted).unwrap();
        assert_eq!(rule, program, "{written}");
        assert_eq!(initial, configuration, "{written}");
    }
}

#[test]
fn name() {
    assert!(matches!(
        Vocabulary::try_from(vec!["A".to_owned(), "A".to_owned()]),
        Err(crate::failure::Failure::Duplicate { .. })
    ));
    for name in ["", "A.B", "A B", "[A]"] {
        assert!(matches!(
            Vocabulary::try_from(vec![name.to_owned()]),
            Err(crate::failure::Failure::Name { .. })
        ));
    }
    let mut vocabulary = Vocabulary::default();
    let atom = vocabulary.intern("人").unwrap();
    assert_eq!(vocabulary.intern("人").unwrap(), atom);
    assert_eq!(vocabulary.name(atom), "人");
}
