use crate::address::Address;
use crate::failure::Failure;
use crate::format::Format;

pub fn unsigned(radix: u8, width: usize, value: u64) -> Result<String, Failure> {
    let format = Format::new(radix, width)?;
    let word = word(format, "Digit", value)?;
    if width == 1 {
        return Ok(format!("().{word}\n"));
    }
    Ok(word + "\n")
}

fn word(format: Format, label: &'static str, value: u64) -> Result<String, Failure> {
    format.check(value)?;
    let radix = format.base();
    let width = format.width();
    Ok((0..width)
        .map(|index| {
            Address::at(label, index)
                .field(((value as u128 / (radix as u128).pow(index as u32)) % radix as u128) as u8)
        })
        .collect::<Vec<_>>()
        .join("."))
}

pub fn difference(radix: u8, width: usize, value: i128) -> Result<String, Failure> {
    let format = Format::new(radix, width)?;
    let magnitude = u64::try_from(value.unsigned_abs()).map_err(|_| Failure::Capacity)?;
    Ok(format!(
        "{}.{}\n",
        word(format, "Digit", magnitude)?,
        Address::scalar("Negative").field(u8::from(value < 0))
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
        "{}.{}.{}\n",
        word(format, "Quotient", value)?,
        word(format, "Remainder", remainder)?,
        Address::scalar("Undefined").field(u8::from(undefined))
    ))
}
