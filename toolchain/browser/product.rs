use crate::failure::{Code, Failure};
use photonic::path::Search;
use photonic::runtime::Limit;
use serde::Serialize;

#[derive(Serialize)]
struct Pair {
    left: u8,
    right: u8,
    digit: u8,
    carry: u8,
}

#[derive(Serialize)]
struct Column {
    contribution: u8,
    incoming: u8,
    digit: u8,
    carry: u8,
}

fn apply(operation: &str, left: u8, right: u8) -> Result<(u8, u8), Failure> {
    let source = format!(
        "Function.{operation}.{left}.{right}\n{}",
        include_str!(env!("DIGIT"))
    );
    let program =
        photonic::lowering::parse(&source).map_err(|error| Failure::new(Code::Source, error))?;
    let mut search = Search::new(program, Default::default())
        .map_err(|error| Failure::new(Code::Source, error))?;
    search.run(1000, Limit::default());
    let state = search.current();
    let field = |name| {
        state
            .world
            .iter()
            .flat_map(|world| &world.particle)
            .find_map(|token| {
                token
                    .display
                    .strip_prefix(&format!("⟨[{name}] "))?
                    .strip_suffix('⟩')?
                    .parse::<u8>()
                    .ok()
            })
            .ok_or_else(|| {
                Failure::new(Code::Source, "The native digit operation did not complete.")
            })
    };
    Ok((field("Digit")?, field("Carry")?))
}

fn reduce(input: &[u8]) -> Result<(u8, u8), Failure> {
    let mut digit = 0;
    let mut carry = 0;
    for &value in input {
        let result = apply("Add", digit, value)?;
        digit = result.0;
        carry = apply("Add", carry, result.1)?.0;
    }
    Ok((digit, carry))
}

pub fn run(left: u8, right: u8) -> Result<serde_json::Value, Failure> {
    if left > 8 || right > 8 {
        return Err(Failure::new(
            Code::Request,
            "Use two-trit operands from 0 through 8.",
        ));
    }
    let mut pair = Vec::new();
    let mut contribution = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for (index, left) in [left % 3, left / 3].into_iter().enumerate() {
        for (position, right) in [right % 3, right / 3].into_iter().enumerate() {
            let (digit, carry) = apply("Multiply", left, right)?;
            pair.push(Pair {
                left,
                right,
                digit,
                carry,
            });
            contribution[index + position].push(digit);
            contribution[index + position + 1].push(carry);
        }
    }
    let mut column = Vec::new();
    let mut incoming = 0;
    for mut input in contribution {
        let subtotal = reduce(&input)?;
        input.push(incoming);
        let (digit, carry) = reduce(&input)?;
        column.push(Column {
            contribution: subtotal.0 + 3 * subtotal.1,
            incoming,
            digit,
            carry,
        });
        incoming = carry;
    }
    let ternary = column
        .iter()
        .rev()
        .map(|value| char::from(b'0' + value.digit))
        .collect::<String>();
    let ternary = ternary.trim_start_matches('0');
    Ok(
        serde_json::json!({"pair": pair, "column": column, "ternary": if ternary.is_empty() { "0" } else { ternary }}),
    )
}
