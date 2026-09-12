pub fn digit(value: u8) -> &'static str {
    match value {
        0 => "Zero",
        1 => "One",
        2 => "Two",
        _ => unreachable!("digit outside supported radix"),
    }
}

pub fn label(radix: u8) -> &'static str {
    match radix {
        2 => "Bit",
        3 => "Trit",
        _ => unreachable!("unsupported radix"),
    }
}

pub fn unsigned(radix: u8, width: usize, value: u64) -> String {
    word(radix, label(radix), width, value) + "\n"
}

fn word(radix: u8, label: &str, width: usize, value: u64) -> String {
    assert!((2..=3).contains(&radix) && width >= 1 && width <= if radix == 2 { 64 } else { 40 });
    assert!((value as u128) < (radix as u128).pow(width as u32));
    (0..width)
        .map(|index| {
            format!(
                "{label}{index}{}",
                digit(((value as u128 / (radix as u128).pow(index as u32)) % radix as u128) as u8)
            )
        })
        .collect::<Vec<_>>()
        .join(".")
}

pub fn difference(radix: u8, width: usize, value: i128) -> String {
    assert!(value.unsigned_abs() <= u64::MAX as u128);
    format!(
        "{}.Negative{}\n",
        word(radix, label(radix), width, value.unsigned_abs() as u64),
        digit(u8::from(value < 0))
    )
}

pub fn quotient(radix: u8, width: usize, value: u64, remainder: u64, undefined: bool) -> String {
    format!(
        "{}.{}.Undefined{}\n",
        word(radix, "Quotient", width, value),
        word(radix, "Remainder", width, remainder),
        digit(u8::from(undefined))
    )
}
