use pest::{Parser, Token, error::InputLocation};

use crate::failure::Failure;
use crate::syntax::{Kind, Node, Tree};

#[derive(pest_derive::Parser)]
#[grammar_inline = r#"
module = { SOI ~ item* ~ EOI }
item = _{ concept | group | context | continuation | coherence | space }
group = { "(" ~ item* ~ ")" }
context = { "[" ~ item* ~ "]" }
continuation = { "." }
coherence = { "," }
space = @{ (" " | "\t" | "\r" | "\n" | "\u{000B}" | "\u{000C}")+ }
concept = @{ (!("(" | ")" | "[" | "]" | "." | "," | space) ~ ANY)+ }
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
            Token::Start { rule, pos } => {
                let kind = match rule {
                    Rule::module => Kind::Module,
                    Rule::concept => Kind::Concept,
                    Rule::group => Kind::Group,
                    Rule::context => Kind::Context,
                    Rule::continuation => Kind::Continuation,
                    Rule::coherence => Kind::Coherence,
                    Rule::space => Kind::Space,
                    Rule::item | Rule::EOI => unreachable!(),
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
    let limit = DEPTH;
    let mut depth = 0usize;
    for (position, byte) in source.bytes().enumerate() {
        match byte {
            b'(' | b'[' => {
                depth += 1;
                if depth > limit {
                    return Err(Failure::Depth {
                        limit,
                        span: (position, 1).into(),
                    });
                }
            }
            b')' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}
