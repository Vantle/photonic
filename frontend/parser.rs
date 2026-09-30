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
atom = @{ (!("(" | ")" | "[" | "]" | "." | "," | WHITESPACE) ~ ANY)+ }
WHITESPACE = _{ PATTERN_WHITE_SPACE }
"#]
struct Grammar;

const SPACE: [char; 11] = [
    '\t', '\n', '\u{000B}', '\u{000C}', '\r', ' ', '\u{0085}', '\u{200E}', '\u{200F}', '\u{2028}',
    '\u{2029}',
];
const DELIMITER: [char; 6] = ['(', ')', '[', ']', '.', ','];

pub fn separator(character: char) -> bool {
    SPACE.contains(&character) || DELIMITER.contains(&character)
}

pub fn parse(source: &str) -> Result<Tree<'_>, Failure> {
    depth(source)?;
    let parsed = Grammar::parse(Rule::module, source).map_err(|error| {
        let position = match error.location {
            InputLocation::Pos(position) | InputLocation::Span((position, _)) => position,
        };
        let length = source[position..].chars().next().map_or(0, char::len_utf8);
        let message = if length == 0 {
            "close what is still open"
        } else {
            "this closes nothing that is open here"
        };
        Failure::Syntax {
            message: message.into(),
            span: (position, length).into(),
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
                    Rule::bracket => Kind::Bracket,
                    Rule::atom => Kind::Atom,
                    Rule::WHITESPACE | Rule::EOI => unreachable!(),
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

pub const DEPTH: usize = 128;

fn depth(source: &str) -> Result<(), Failure> {
    let mut level = 0usize;
    for (position, byte) in source.bytes().enumerate() {
        match byte {
            b'(' | b'[' => {
                level += 1;
                if level > DEPTH {
                    return Err(Failure::Depth {
                        limit: DEPTH,
                        span: (position, 1).into(),
                    });
                }
            }
            b')' | b']' => level = level.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}
