use pest::{Parser, Token, error::InputLocation};

use crate::failure::Failure;
use crate::syntax::{Kind, Node, Tree};

#[derive(pest_derive::Parser)]
#[grammar_inline = r#"
module = { SOI ~ list ~ EOI }
list = { (term ~ ("," ~ term)* ~ ","?)? }
term = { rule+ ~ (join ~ rule*)? | join ~ rule* }
join = _{ factor ~ ("." ~ factor)* }
factor = _{ concept | group }
group = { "(" ~ list ~ ")" }
rule = { "[" ~ list ~ "]" }
concept = @{ (!("(" | ")" | "[" | "]" | "." | "," | WHITESPACE | CONTROL | BIDI_CONTROL | "\u{200B}" | "\u{2060}") ~ ANY)+ }
WHITESPACE = _{ WHITE_SPACE | "\u{FEFF}" }
"#]
struct Grammar;

pub fn parse(source: &str) -> Result<Tree<'_>, Failure> {
    if let Some(position) = depth(source) {
        return Err(match crate::advice::find(source) {
            Some((advised, message)) if advised < position => syntax(source, advised, message),
            _ => Failure::Depth {
                limit: DEPTH,
                span: (position, 1).into(),
            },
        });
    }
    let parsed = Grammar::parse(Rule::module, source).map_err(|error| {
        let position = match error.location {
            InputLocation::Pos(position) | InputLocation::Span((position, _)) => position,
        };
        let (position, message) = crate::advice::find(source)
            .filter(|&(advised, _)| advised <= position)
            .unwrap_or_else(|| (position, error.variant.message().into_owned()));
        syntax(source, position, message)
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
                    Rule::join | Rule::factor | Rule::WHITESPACE | Rule::EOI => unreachable!(),
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
    Ok(Tree::new(source, node))
}

fn syntax(source: &str, position: usize, message: String) -> Failure {
    let length = source[position..].chars().next().map_or(0, char::len_utf8);
    Failure::Syntax {
        message,
        span: (position, length).into(),
    }
}

pub const DEPTH: usize = 128;

// Pest descends once for each level a source nests, so a source is measured before it is parsed.
fn depth(source: &str) -> Option<usize> {
    let mut level = 0usize;
    for (position, byte) in source.bytes().enumerate() {
        match byte {
            b'(' | b'[' => {
                level += 1;
                if level > DEPTH {
                    return Some(position);
                }
            }
            b')' | b']' => level = level.saturating_sub(1),
            _ => {}
        }
    }
    None
}
