use miette::{Diagnostic, SourceSpan};
use pest::{Parser, error::InputLocation, iterators::Pair};
use thiserror::Error;

use crate::source::{Definition, Output, Program, Value};

#[derive(Debug, Diagnostic, Error)]
pub enum Failure {
    #[error("invalid executable Molten syntax: {message}")]
    #[diagnostic(code(molten::lowering))]
    Syntax {
        message: String,
        #[label("{message}")]
        span: SourceSpan,
    },
    #[error("executable syntax nesting exceeds {limit} levels")]
    #[diagnostic(code(molten::depth))]
    Depth {
        limit: usize,
        #[label("nesting limit exceeded here")]
        span: SourceSpan,
    },
    #[error("a body accepts at most one initial coherence; found {count}")]
    #[diagnostic(code(molten::body))]
    Body {
        count: usize,
        #[label("use one particle as the body's explicit initial output")]
        span: SourceSpan,
    },
}

#[derive(pest_derive::Parser)]
#[grammar_inline = r#"
WHITESPACE = _{ " " | "\t" | "\r" | "\n" | "\u{000B}" | "\u{000C}" }
module = { SOI ~ sequence ~ EOI }
sequence = _{ initial ~ (definition ~ ending)* | (definition ~ ending)+ | &("}" | EOI) }
initial = { configuration ~ ending }
ending = { ";" }
configuration = { particle ~ ("," ~ particle)* }
definition = { input ~ negative? ~ "->" ~ output }
input = { "[" ~ configuration? ~ "]" }
negative = { "unless" ~ input }
output = { discard | destination ~ ("," ~ destination)* }
discard = { "[" ~ "]" }
destination = _{ body | particle }
body = { "{" ~ sequence ~ "}" }
particle = { empty | value ~ ("." ~ value)* }
empty = { "(" ~ ")" }
value = _{ closure | variable | structure | atom }
variable = @{ "$" ~ atom }
structure = { atom ~ "(" ~ particle? ~ closing }
closing = { ")" }
closure = { "@" ~ "(" ~ definition ~ ")" }
atom = @{ (!("->" | "." | "," | "[" | "]" | "(" | ")" | "{" | "}" | "@" | "$" | ";" | WHITESPACE) ~ ANY)+ }
"#]
struct Grammar;

pub fn parse(source: &str) -> Result<Program, Failure> {
    depth(source)?;
    let mut parsed = Grammar::parse(Rule::module, source).map_err(|error| {
        let span = match error.location {
            InputLocation::Pos(position) => {
                let length = source[position..].chars().next().map_or(0, char::len_utf8);
                (position, length).into()
            }
            InputLocation::Span((start, end)) => (start, end - start).into(),
        };
        Failure::Syntax {
            message: error.variant.message().into_owned(),
            span,
        }
    })?;
    program(parsed.next().expect("parsed module"))
}

fn program(parsed: Pair<'_, Rule>) -> Result<Program, Failure> {
    let mut initial = Vec::new();
    let mut rule = Vec::new();
    for item in parsed.into_inner() {
        match item.as_rule() {
            Rule::initial => {
                initial = configuration(item.into_inner().next().expect("initial configuration"))?;
            }
            Rule::definition => rule.push(definition(item)?),
            Rule::EOI | Rule::ending => {}
            _ => unreachable!(),
        }
    }
    Ok(Program { initial, rule })
}

fn definition(parsed: Pair<'_, Rule>) -> Result<Definition, Failure> {
    let name = parsed.as_str().trim().to_owned();
    let mut child = parsed.into_inner().peekable();
    let input = pattern(child.next().expect("rule input"))?;
    let negative = if child
        .peek()
        .is_some_and(|item| item.as_rule() == Rule::negative)
    {
        Some(pattern(
            child
                .next()
                .expect("negative premise")
                .into_inner()
                .next()
                .expect("negative input"),
        )?)
    } else {
        None
    };
    let output = output(child.next().expect("rule output"))?;
    Ok(Definition {
        name,
        input,
        output,
        negative,
    })
}

fn pattern(parsed: Pair<'_, Rule>) -> Result<Vec<Vec<Value>>, Failure> {
    parsed
        .into_inner()
        .next()
        .map_or_else(|| Ok(Vec::new()), configuration)
}

fn configuration(parsed: Pair<'_, Rule>) -> Result<Vec<Vec<Value>>, Failure> {
    parsed.into_inner().map(particle).collect()
}

fn particle(parsed: Pair<'_, Rule>) -> Result<Vec<Value>, Failure> {
    parsed
        .into_inner()
        .filter(|item| item.as_rule() != Rule::empty)
        .map(value)
        .collect()
}

fn value(parsed: Pair<'_, Rule>) -> Result<Value, Failure> {
    match parsed.as_rule() {
        Rule::atom => Ok(Value::Atom(parsed.as_str().to_owned())),
        Rule::variable => Ok(Value::Variable {
            variable: parsed.as_str()[1..].to_owned(),
        }),
        Rule::structure => {
            let mut child = parsed.into_inner();
            let structure = child.next().expect("structure name").as_str().to_owned();
            let particle = child
                .find(|item| item.as_rule() == Rule::particle)
                .map_or_else(|| Ok(Vec::new()), particle)?;
            Ok(Value::Structure {
                structure,
                particle,
            })
        }
        Rule::closure => Ok(Value::Rule {
            rule: Box::new(definition(parsed.into_inner().next().expect("rule value"))?),
        }),
        _ => unreachable!(),
    }
}

fn output(parsed: Pair<'_, Rule>) -> Result<Vec<Output>, Failure> {
    parsed
        .into_inner()
        .filter(|item| item.as_rule() != Rule::discard)
        .map(|item| match item.as_rule() {
            Rule::body => body(item),
            Rule::particle => Ok(Output {
                particle: particle(item)?,
                body: None,
            }),
            _ => unreachable!(),
        })
        .collect()
}

fn body(parsed: Pair<'_, Rule>) -> Result<Output, Failure> {
    let span = (parsed.as_span().start(), parsed.as_str().len()).into();
    let source = program(parsed)?;
    if source.initial.len() > 1 {
        return Err(Failure::Body {
            count: source.initial.len(),
            span,
        });
    }
    Ok(Output {
        particle: source.initial.into_iter().next().unwrap_or_default(),
        body: Some(source.rule),
    })
}

fn depth(source: &str) -> Result<(), Failure> {
    let limit = 128;
    let mut depth = 0usize;
    for (position, byte) in source.bytes().enumerate() {
        match byte {
            b'(' | b'[' | b'{' => {
                depth += 1;
                if depth > limit {
                    return Err(Failure::Depth {
                        limit,
                        span: (position, 1).into(),
                    });
                }
            }
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}
