use crate::failure::Failure;

fn magnitude(source: &str) -> Result<u128, Failure> {
    let (radix, digit) = match source.as_bytes().get(..2) {
        Some(b"0b" | b"0B") => (2, &source[2..]),
        Some(b"0t" | b"0T") => (3, &source[2..]),
        Some(b"0o" | b"0O") => (8, &source[2..]),
        Some(b"0x" | b"0X") => (16, &source[2..]),
        _ => (10, source),
    };
    if digit.is_empty()
        || digit.split('_').any(|part| {
            part.is_empty()
                || !part
                    .chars()
                    .all(|value| value.is_ascii() && value.is_digit(radix))
        })
    {
        return Err(Failure::Syntax);
    }
    u128::from_str_radix(&digit.replace('_', ""), radix).map_err(|_| Failure::Range)
}

pub fn unsigned(source: &str) -> Result<u64, Failure> {
    if source.starts_with('-') {
        return Err(Failure::Sign);
    }
    magnitude(source.strip_prefix('+').unwrap_or(source))?
        .try_into()
        .map_err(|_| Failure::Range)
}

pub fn signed(source: &str) -> Result<i128, Failure> {
    let negative = source.starts_with('-');
    let digit = source.strip_prefix(['-', '+']).unwrap_or(source);
    let value = magnitude(digit)?;
    if negative && value == (1u128 << 127) {
        return Ok(i128::MIN);
    }
    let value = i128::try_from(value).map_err(|_| Failure::Range)?;
    Ok(if negative { -value } else { value })
}
