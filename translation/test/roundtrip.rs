use super::support::{configuration, flat, nested, vocabulary};
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
        let parsed = photonic::lowering::parse(&written)
            .unwrap_or_else(|failure| panic!("{failure}: {written}"));
        let mut lifted = vocabulary();
        let (rule, initial) = lift::program(&parsed, &mut lifted).unwrap();
        assert_eq!(rule, program, "{written}");
        assert_eq!(initial, configuration, "{written}");
    }
}

#[test]
fn rest() {
    let parsed = photonic::lowering::parse("[A] [B] C").unwrap();
    assert_eq!(
        lift::program(&parsed, &mut vocabulary()).unwrap_err(),
        crate::failure::Failure::Rest {
            name: "[A] [B] C".to_owned()
        }
    );
}

#[test]
fn unknown() {
    let parsed = photonic::lowering::parse("[Seed] ().([Missing] B)").unwrap();
    let known = vocabulary();
    assert!(matches!(
        lift::program(&parsed, &mut lift::Fixed(&known)),
        Err(crate::failure::Failure::Unknown { .. })
    ));
}
