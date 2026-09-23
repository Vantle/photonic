use crate::failure::{Code, Failure};
use photonic::path::Search;
use photonic::runtime::Limit;

fn encode(input: &str) -> Result<String, Failure> {
    if input.len() > 256 {
        return Err(Failure::new(
            Code::Size,
            "Use at most 256 bytes of expression input.",
        ));
    }
    let mut token = Vec::new();
    let mut gap = false;
    for character in input.chars() {
        if character.is_whitespace() {
            gap = true;
            continue;
        }
        if gap
            && matches!(character, '0'..='2')
            && token
                .last()
                .is_some_and(|value: &String| value.starts_with("([Digit]"))
        {
            return Err(Failure::new(
                Code::Source,
                "Keep each numeral together; spaces separate operators, not digits.",
            ));
        }
        gap = false;
        token.push(match character {
            '0'..='2' => format!("([Digit] {character})"),
            '+' => "Add".into(),
            '-' | '−' => "Subtract".into(),
            '*' | '×' => "Multiply".into(),
            '/' | '÷' => "Divide".into(),
            '(' => "Open".into(),
            ')' => "Close".into(),
            _ => {
                return Err(Failure::new(
                    Code::Source,
                    "Use base-three digits 0, 1, 2, operators + − × ÷, and parentheses.",
                ));
            }
        });
    }
    token.reverse();
    let Some(first) = token.first() else {
        return Ok("Function.Expression.Evaluate.Zero".into());
    };
    let mut source = format!("Push.{first}.Zero, Stage.1\n");
    for (index, value) in token.iter().enumerate().skip(1) {
        source.push_str(&format!(
            "[Built, {}] (Push.{value}) (Forget.Stage.{})\n",
            stage(index),
            index + 1
        ));
    }
    source.push_str(&format!(
        "[Built, {}] Function.Expression.Evaluate\n",
        stage(token.len())
    ));
    Ok(source)
}

fn stage(index: usize) -> String {
    if index == 1 {
        return "Stage.1".into();
    }
    format!("Clean.Stage.{index}")
}

static FORMULA: std::sync::LazyLock<Result<photonic::source::Program, String>> =
    std::sync::LazyLock::new(|| {
        serde_json::from_str(include_str!(env!("FORMULA"))).map_err(|error| error.to_string())
    });

pub fn prepare(input: &str) -> Result<(Search, String), Failure> {
    let source = encode(input)?;
    let mut program = FORMULA
        .as_ref()
        .map_err(|error| Failure::new(Code::Source, error))?
        .clone();
    let encoded =
        photonic::lowering::parse(&source).map_err(|error| Failure::new(Code::Source, error))?;
    program.initial = encoded.initial;
    program.rule.extend(encoded.rule);
    let search = Search::new(program, photonic::source::Program::default());
    Ok((search, source))
}

pub fn advance(search: &mut Search) {
    search.run(
        1000000,
        Limit {
            state: 32768,
            cell: 8192,
            frame: 1024,
            world: 512,
            record: 2000000,
        },
    );
}

pub fn run(input: &str) -> Result<serde_json::Value, Failure> {
    let (mut search, source) = prepare(input)?;
    advance(&mut search);
    let summary = search.summary();
    Ok(
        serde_json::json!({"version": 1, "state": search.current(), "work": summary.work, "event": summary.event, "source": source}),
    )
}
