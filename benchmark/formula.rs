use frontend::source::Program;

pub fn source(input: &str, expected: &str) -> Result<String, Box<dyn std::error::Error>> {
    let (sign, magnitude) = match expected.strip_prefix('-') {
        Some(magnitude) => ("Negative", magnitude),
        None => ("Positive", expected),
    };
    if magnitude.is_empty() || !magnitude.chars().all(|digit| matches!(digit, '0'..='2')) {
        return Err("use a ternary expected result, such as 21, 0 or -102".into());
    }
    let digit = magnitude.trim_start_matches('0');
    let sign = if digit.is_empty() { "Positive" } else { sign };
    let mut source = infix::tape(input, "(Function.Expression.Evaluate, Forget.Inspect.0)")?;
    source.push_str(&format!(
        "[Return.Expression.Evaluate.{sign}, Clean.Inspect.0] (Read, Forget.Inspect.1),\n"
    ));
    if digit.is_empty() {
        source.push_str("[Yield.End.Zero, Clean.Inspect.1] Done.Zero,\n");
        return Ok(source);
    }
    for (index, value) in digit.chars().rev().enumerate() {
        let current = index + 1;
        if current == digit.len() {
            source.push_str(&format!(
                "[Yield.([Digit] {value}).Zero, Clean.Inspect.{current}] Done.Zero,\n"
            ));
        } else {
            source.push_str(&format!(
                "[Yield.([Digit] {value}), Clean.Inspect.{current}] (Read, Forget.Inspect.{}),\n",
                current + 1
            ));
        }
    }
    Ok(source)
}

pub fn program(source: &str) -> Result<(Program, Program), Box<dyn std::error::Error>> {
    let mut program = Program::read(include_str!(env!("FORMULA")))?;
    let encoded = frontend::lowering::parse(source)?;
    program.initial = encoded.initial;
    program.rule.extend(encoded.rule);
    let mut target = frontend::lowering::parse("Done.Zero")?;
    target.preserve(&program);
    Ok((program, target))
}
