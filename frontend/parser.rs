use pest::{Parser, Token, error::InputLocation};

use crate::failure::Failure;
use crate::syntax::{Kind, Node, Tree};

#[derive(pest_derive::Parser)]
#[grammar_inline = r#"
module = { SOI ~ list ~ EOI }
list = { term* ~ ("," ~ term*)* }
term = { factor ~ ("." ~ factor)* }
factor = _{ concept | group | rule }
group = { "(" ~ list ~ ")" }
rule = { "[" ~ list ~ "]" ~ term* }
concept = @{ (!("(" | ")" | "[" | "]" | "." | "," | WHITESPACE) ~ ANY)+ }
WHITESPACE = _{ " " | "\t" | "\r" | "\n" | "\u{000B}" | "\u{000C}" }
"#]
struct Grammar;

pub fn parse(source: &str) -> Result<Tree<'_>, Failure> {
    depth(source)?;
    let parsed = Grammar::parse(Rule::module, source).map_err(|error| {
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
    let mut node = Vec::new();
    let mut stack = Vec::new();
    for token in parsed.tokens() {
        match token {
            Token::Start {
                rule: Rule::EOI, ..
            }
            | Token::End {
                rule: Rule::EOI, ..
            } => {}
            Token::Start { rule: name, pos } => {
                let kind = match name {
                    Rule::module => Kind::Module,
                    Rule::list => Kind::List,
                    Rule::term => Kind::Term,
                    Rule::group => Kind::Group,
                    Rule::rule => Kind::Rule,
                    Rule::concept => Kind::Concept,
                    Rule::factor | Rule::WHITESPACE | Rule::EOI => unreachable!(),
                };
                let index = node.len();
                node.push(Node {
                    kind,
                    span: pos.pos()..pos.pos(),
                    parent: stack.last().copied(),
                });
                stack.push(index);
            }
            Token::End { pos, .. } => {
                let index = stack.pop().expect("balanced parser token stream");
                node[index].span.end = pos.pos();
            }
        }
    }
    Ok(Tree { source, node })
}

pub const DEPTH: usize = 128;

fn depth(source: &str) -> Result<(), Failure> {
    let mut output = vec![0usize];
    let mut level = 0usize;
    for (position, byte) in source.bytes().enumerate() {
        match byte {
            b'(' | b'[' => {
                output.push(0);
                level += 1;
            }
            b')' | b']' => {
                if output.len() > 1 {
                    level -= 1 + output.pop().unwrap_or_default();
                }
                if byte == b')' {
                    continue;
                }
                if let Some(open) = output.last_mut() {
                    *open += 1;
                }
                level += 1;
            }
            b',' => {
                if let Some(open) = output.last_mut() {
                    level -= *open;
                    *open = 0;
                }
                continue;
            }
            _ => continue,
        }
        if level > DEPTH {
            return Err(Failure::Depth {
                limit: DEPTH,
                span: (position, 1).into(),
            });
        }
    }
    Ok(())
}
