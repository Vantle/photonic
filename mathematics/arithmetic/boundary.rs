use arithmetic::{circuit, encoding, failure::Failure, power};

#[test]
fn circuit() {
    for generate in [circuit::add, circuit::subtract, circuit::divide] {
        for base in [0, 1, 4, u8::MAX] {
            assert!(matches!(generate(base, 1, 0, 0), Err(Failure::Base { .. })));
        }
        for (base, maximum) in [(2, 32), (3, 20)] {
            for width in [0, maximum + 1, usize::MAX] {
                assert!(matches!(
                    generate(base, width, 0, 0),
                    Err(Failure::Width { .. })
                ));
            }
            assert_eq!(generate(base, 1, base.into(), 0), Err(Failure::Capacity));
            assert_eq!(generate(base, 1, 0, base.into()), Err(Failure::Capacity));
        }
    }
    for layout in [circuit::Layout::Column, circuit::Layout::Balanced] {
        assert!(matches!(
            circuit::multiply(0, 1, 0, 0, layout),
            Err(Failure::Base { .. })
        ));
        assert!(matches!(
            circuit::multiply(2, 0, 0, 0, layout),
            Err(Failure::Width { .. })
        ));
        assert_eq!(
            circuit::multiply(2, 1, 2, 0, layout),
            Err(Failure::Capacity)
        );
    }
}

#[test]
fn encoding() {
    assert!(encoding::unsigned(2, 64, u64::MAX).is_ok());
    assert!(encoding::unsigned(3, 40, 3_u64.pow(40) - 1).is_ok());
    assert_eq!(
        encoding::unsigned(3, 40, 3_u64.pow(40)),
        Err(Failure::Capacity)
    );
    for value in [i128::MIN, i128::MAX] {
        assert_eq!(encoding::difference(2, 64, value), Err(Failure::Capacity));
    }
    for base in [0, 1, 4, u8::MAX] {
        assert!(matches!(
            encoding::unsigned(base, 1, 0),
            Err(Failure::Base { .. })
        ));
        assert!(matches!(
            encoding::difference(base, 1, 0),
            Err(Failure::Base { .. })
        ));
        assert!(matches!(
            encoding::quotient(base, 1, 0, 0, false),
            Err(Failure::Base { .. })
        ));
    }
    for (base, maximum) in [(2, 64), (3, 40)] {
        for width in [0, maximum + 1, usize::MAX] {
            assert!(matches!(
                encoding::unsigned(base, width, 0),
                Err(Failure::Width { .. })
            ));
        }
    }
    assert_eq!(
        encoding::quotient(2, 1, 2, 0, false),
        Err(Failure::Capacity)
    );
    assert_eq!(
        encoding::quotient(2, 1, 0, 2, false),
        Err(Failure::Capacity)
    );
}

#[test]
fn power() {
    for base in [0, 1, 17, u32::MAX] {
        assert!(matches!(power::numeral(1, base), Err(Failure::Base { .. })));
        assert!(matches!(power::rule(base, 1), Err(Failure::Base { .. })));
    }
    assert_eq!(power::rule(2, 0).unwrap(), "");
    assert!(power::rule(16, 64).is_ok());
    assert!(matches!(
        power::rule(2, usize::MAX),
        Err(Failure::Width { .. })
    ));
    assert!(power::numeral(u64::MAX, 2).is_ok());
}
