use super::{Hasher, combine, mix, value};

#[test]
fn golden() {
    assert_eq!(mix(0), 0);
    assert_eq!(mix(1), 0x5692_161d_100b_05e5);
    assert_eq!(mix(u64::MAX), 0xb4d0_55fc_f2cb_bd7b);
    assert_eq!(combine(1, 2), 0xfe6f_a0f2_f7af_a5b8);
    assert_eq!(value(&42_u64), 0xae70_bf69_4a4b_18a9);
    assert_eq!(value(&"photonic"), 0xce63_1e4a_cca0_d4ec);
}

fn written(byte: &[u8]) -> u64 {
    let mut hasher = Hasher::default();
    std::hash::Hasher::write(&mut hasher, byte);
    std::hash::Hasher::finish(&hasher)
}

// Bytes that differ only in trailing zeros fill the same padded words, so only their length tells
// them apart.
#[test]
fn length() {
    assert_ne!(value(&"A"), value(&"A\0"));
    assert_ne!(value(&""), value(&"\0"));
    assert_ne!(written(b"photonic"), written(b"photonic\0"));
    assert_ne!(written(b""), written(b"\0\0\0"));
}
