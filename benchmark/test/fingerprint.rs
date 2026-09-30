use super::Fingerprint;

fn fingerprint(byte: &[u8]) -> u64 {
    let mut fingerprint = Fingerprint::default();
    fingerprint.update(byte);
    fingerprint.value()
}

#[test]
fn vector() {
    assert_eq!(fingerprint(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fingerprint(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(fingerprint(b"foobar"), 0x8594_4171_f739_67e8);
    let mut streamed = Fingerprint::default();
    streamed.update(b"foo");
    streamed.update(b"bar");
    assert_eq!(streamed.value(), fingerprint(b"foobar"));
}
