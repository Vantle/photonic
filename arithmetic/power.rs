use crate::failure::Failure;

pub fn numeral(mut value: u64, radix: u32) -> Result<String, Failure> {
    base(radix)?;
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
    Ok(if particle.is_empty() {
        "()".into()
    } else {
        particle.join(".")
    })
}

pub fn rule(radix: u32, width: usize) -> Result<String, Failure> {
    base(radix)?;
    if width > 64 {
        return Err(Failure::Width {
            value: width,
            minimum: 0,
            maximum: 64,
        });
    }
    Ok((0..width)
        .map(|position| {
            let input = vec![format!("{radix}^{position}"); radix as usize].join(".");
            format!("[{input}] {radix}^{},\n", position + 1)
        })
        .collect())
}

fn base(value: u32) -> Result<(), Failure> {
    if !(2..=16).contains(&value) {
        return Err(Failure::Base {
            value,
            minimum: 2,
            maximum: 16,
        });
    }
    Ok(())
}
