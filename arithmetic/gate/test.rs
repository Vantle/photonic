use super::Kind;

#[test]
fn borrow() {
    for mask in 0..8 {
        let output = Kind::Difference.evaluate(
            2,
            &[
                (mask & 1) as u8,
                ((mask >> 1) & 1) as u8,
                ((mask >> 2) & 1) as u8,
            ],
        );
        let left = mask & 1;
        let right = (mask >> 1) & 1;
        let incoming = (mask >> 2) & 1;
        assert_eq!(
            left - right - incoming,
            output[0] as i32 - 2 * output[1] as i32
        );
    }
}

#[test]
fn ternary() {
    for left in 0..3 {
        for right in 0..3 {
            for borrow in 0..2 {
                let output = Kind::Difference.evaluate(3, &[left, right, borrow]);
                assert_eq!(
                    left as i32 - right as i32 - borrow as i32,
                    output[0] as i32 - 3 * output[1] as i32
                );
            }
        }
    }
}
