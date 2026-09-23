pub fn source(input: &str, expected: &str) -> Result<String, Box<dyn std::error::Error>> {
    let token = infix::token(input)?;
    let mut source = infix::stack(&token, "(Function.Expression.Evaluate) (Forget.Inspect.0)")
        .ok_or("use a nonempty expression")?;
    source.push_str(
        "[Return.Expression.Evaluate.Positive, Clean.Inspect.0] (Read) (Forget.Inspect.1)\n",
    );
    for (index, digit) in expected.chars().rev().enumerate() {
        let current = index + 1;
        if current == expected.len() {
            source.push_str(&format!(
                "[Yield.([Digit] {digit}).Zero, Clean.Inspect.{current}] Done.Zero\n"
            ));
        } else {
            source.push_str(&format!(
                "[Yield.([Digit] {digit}), Clean.Inspect.{current}] (Read) (Forget.Inspect.{})\n",
                current + 1
            ));
        }
    }
    Ok(source)
}
