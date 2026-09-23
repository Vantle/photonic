use crate::failure::{Code, Failure};
use photonic::path::Search;
use photonic::runtime::Limit;
use photonic::snapshot::Node;
use serde::Serialize;

#[derive(Serialize)]
pub struct Calculation {
    state: Node,
    work: usize,
    event: usize,
    source: String,
}

impl Calculation {
    pub fn new(search: &Search, source: String) -> Self {
        let summary = search.summary();
        Self {
            state: search.current(),
            work: summary.work,
            event: summary.event,
            source,
        }
    }
}

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

pub fn run(input: &str) -> Result<Calculation, Failure> {
    let (mut search, source) = prepare(input)?;
    advance(&mut search);
    Ok(Calculation::new(&search, source))
}
