use crate::failure::Failure;
use crate::format::Format;

pub(crate) fn digit(value: u8) -> &'static str {
    match value {
        0 => "Zero",
        1 => "One",
        2 => "Two",
        _ => unreachable!("digit outside supported radix"),
    }
}

pub(crate) fn label(radix: u8) -> &'static str {
    match radix {
        2 => "Bit",
        3 => "Digit",
        _ => unreachable!("unsupported radix"),
    }
}

pub fn unsigned(radix: u8, width: usize, value: u64) -> Result<String, Failure> {
    let format = Format::new(radix, width)?;
    Ok(word(format, label(radix), value)? + "\n")
}

fn word(format: Format, label: &str, value: u64) -> Result<String, Failure> {
    format.check(value)?;
    let radix = format.base();
    let width = format.width();
    Ok((0..width)
        .map(|index| {
            format!(
                "{label}{index}{}",
                digit(((value as u128 / (radix as u128).pow(index as u32)) % radix as u128) as u8)
            )
        })
        .collect::<Vec<_>>()
        .join("."))
}

pub fn difference(radix: u8, width: usize, value: i128) -> Result<String, Failure> {
    let format = Format::new(radix, width)?;
    let magnitude = u64::try_from(value.unsigned_abs()).map_err(|_| Failure::Capacity)?;
    Ok(format!(
        "{}.Negative{}\n",
        word(format, label(radix), magnitude)?,
        digit(u8::from(value < 0))
    ))
}

pub fn quotient(
    radix: u8,
    width: usize,
    value: u64,
    remainder: u64,
    undefined: bool,
) -> Result<String, Failure> {
    let format = Format::new(radix, width)?;
    Ok(format!(
        "{}.{}.Undefined{}\n",
        word(format, "Quotient", value)?,
        word(format, "Remainder", remainder)?,
        digit(u8::from(undefined))
    ))
}
