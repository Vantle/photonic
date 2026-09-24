use crate::failure::{Code, Failure};
use photonic::source::Program;

fn encode(input: &str) -> Result<String, Failure> {
    if input.len() > 256 {
        return Err(Failure::new(
            Code::Size,
            "Use at most 256 bytes of expression input.",
        ));
    }
    let token = infix::token(input).map_err(|rejection| Failure::new(Code::Source, rejection))?;
    Ok(infix::stack(&token, "Function.Expression.Evaluate")
        .unwrap_or_else(|| "Function.Expression.Evaluate.Zero".into()))
}

static FORMULA: std::sync::LazyLock<Result<Program, String>> = std::sync::LazyLock::new(|| {
    serde_json::from_str(include_str!(env!("FORMULA"))).map_err(|error| error.to_string())
});

pub fn prepare(input: &str) -> Result<(Program, String), Failure> {
    let source = encode(input)?;
    let mut program = FORMULA
        .as_ref()
        .map_err(|error| Failure::new(Code::Source, error))?
        .clone();
    let encoded =
        photonic::lowering::parse(&source).map_err(|error| Failure::new(Code::Source, error))?;
    program.initial = encoded.initial;
    program.rule.extend(encoded.rule);
    Ok((program, source))
}
