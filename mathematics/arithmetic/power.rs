pub fn numeral(mut value: u64, radix: u32) -> String {
    assert!((2..=16).contains(&radix));
    let mut particle = Vec::new();
    let mut position = 0;
    while value != 0 {
        particle.extend(std::iter::repeat_n(
            format!("{radix}^{position}"),
            (value % radix as u64) as usize,
        ));
        value /= radix as u64;
        position += 1;
    }
    if particle.is_empty() {
        "()".into()
    } else {
        particle.join(".")
    }
}

pub fn rule(radix: u32, width: usize) -> String {
    assert!((2..=16).contains(&radix) && width <= 64);
    (0..width)
        .map(|position| {
            let input = vec![format!("{radix}^{position}"); radix as usize].join(".");
            format!("[{input}] {radix}^{}\n", position + 1)
        })
        .collect()
}
