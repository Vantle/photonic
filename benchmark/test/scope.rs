#[test]
fn observation() {
    for (width, depth, stride) in [(1, 1, 1), (64, 3, 1), (65, 3, 64), (4097, 2, 4096)] {
        let mut fingerprint = None;
        for length in [5, 6] {
            let value = serde_json::to_value(photonic::measurement::scope::run(
                width, depth, length, stride,
            ))
            .unwrap();
            assert_eq!(value["delivery"], width.div_ceil(stride) * 3);
            let actual = value["fingerprint"].as_u64().unwrap();
            assert_ne!(actual, 0);
            if let Some(expected) = fingerprint {
                assert_eq!(actual, expected);
            }
            fingerprint = Some(actual);
            assert!(value["work"].as_u64().unwrap() > value["delivery"].as_u64().unwrap());
            assert!(value["retained"].as_u64().unwrap() > 0);
        }
    }
}
