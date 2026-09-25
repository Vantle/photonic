use crate::failure::{Code, Failure};
use crate::request::{self, Text};
use photonic::source::Program;

fn encode(source: &str) -> Result<String, Failure> {
    if source.len() > 256 {
        return Err(Failure::new(
            Code::Size,
            "Use at most 256 bytes of expression input.",
        ));
    }
    let token = infix::token(source).map_err(|rejection| Failure::new(Code::Source, rejection))?;
    Ok(infix::stack(&token, "Function.Expression.Evaluate")
        .unwrap_or_else(|| "Function.Expression.Evaluate.Zero".into()))
}

static FORMULA: std::sync::LazyLock<Result<Program, String>> = std::sync::LazyLock::new(|| {
    serde_json::from_str(include_str!(env!("FORMULA"))).map_err(|error| error.to_string())
});

pub fn prepare(input: &str) -> Result<(Program, String), Failure> {
    let text: Text = request::read(input)?;
    let tape = encode(&text.source)?;
    let mut program = FORMULA
        .as_ref()
        .map_err(|error| {
            Failure::new(
                Code::Internal,
                format!("The built-in expression evaluator did not load: {error}"),
            )
        })?
        .clone();
    let encoded = photonic::lowering::parse(&tape).map_err(|error| {
        Failure::new(
            Code::Internal,
            format!("The expression tape did not parse: {error}"),
        )
    })?;
    program.initial = encoded.initial;
    program.rule.extend(encoded.rule);
    Ok((program, tape))
}
