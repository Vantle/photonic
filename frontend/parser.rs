use pest::{Parser, Token, error::InputLocation};

use crate::failure::Failure;
use crate::syntax::{Kind, Node, Tree};

#[derive(pest_derive::Parser)]
#[grammar_inline = r#"
module = { SOI ~ list ~ EOI }
list = { term? ~ ("," ~ term?)* }
term = { (atom | group | bracket | ".")+ }
group = { "(" ~ list ~ ")" }
bracket = { "[" ~ list ~ "]" }
atom = @{ (!("(" | ")" | "[" | "]" | "." | "," | WHITESPACE | SPACE_SEPARATOR | CONTROL | BIDI_CONTROL | "\u{200B}" | "\u{2060}") ~ ANY)+ }
WHITESPACE = _{ PATTERN_WHITE_SPACE | "\u{FEFF}" }
"#]
struct Grammar;

// Every balanced text without a refused character is a program, so the scan that finds the first
// unbalanced bracket or refused character decides, before pest runs, whether the text parses.
pub fn parse(source: &str) -> Result<Tree<'_>, Failure> {
    let problem = crate::advice::find(source);
    if let Some(position) = depth(source) {
        return Err(match problem {
            Some((advised, message)) if advised < position => syntax(source, advised, message),
            _ => Failure::Depth {
                limit: DEPTH,
                span: (position, 1).into(),
            },
        });
    }
    if let Some((position, message)) = problem {
        return Err(syntax(source, position, message));
    }
    let parsed = Grammar::parse(Rule::module, source).map_err(|error| {
        let position = match error.location {
            InputLocation::Pos(position) | InputLocation::Span((position, _)) => position,
        };
        syntax(source, position, error.variant.message().into_owned())
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
                    Rule::bracket => Kind::Bracket,
                    Rule::atom => Kind::Atom,
                    Rule::WHITESPACE | Rule::EOI => unreachable!(),
                };
                stack.push(node.len());
                node.push(Node {
                    kind,
                    span: pos.pos()..pos.pos(),
                    next: 0,
                });
            }
            Token::End { pos, .. } => {
                let index = stack.pop().expect("balanced parser token stream");
                node[index].span.end = pos.pos();
                node[index].next = node.len();
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
