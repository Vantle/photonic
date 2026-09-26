use frontend::source::Program;

pub fn source(input: &str, expected: &str) -> Result<String, Box<dyn std::error::Error>> {
    if expected.is_empty()
        || expected.starts_with('0')
        || !expected.chars().all(|digit| matches!(digit, '0'..='2'))
    {
        return Err("use a positive ternary expected result without leading zeros".into());
    }
    let mut source = infix::tape(input, "(Function.Expression.Evaluate, Forget.Inspect.0)")?;
    source.push_str(
        "[Return.Expression.Evaluate.Positive, Clean.Inspect.0] (Read, Forget.Inspect.1),\n",
    );
    for (index, digit) in expected.chars().rev().enumerate() {
        let current = index + 1;
        if current == expected.len() {
            source.push_str(&format!(
                "[Yield.([Digit] {digit}).Zero, Clean.Inspect.{current}] Done.Zero,\n"
            ));
        } else {
            source.push_str(&format!(
                "[Yield.([Digit] {digit}), Clean.Inspect.{current}] (Read, Forget.Inspect.{}),\n",
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
