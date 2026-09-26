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
concept = @{ (!("(" | ")" | "[" | "]" | "." | "," | WHITESPACE) ~ ANY)+ }
WHITESPACE = _{ " " | "\t" | "\r" | "\n" | "\u{000B}" | "\u{000C}" }
"#]
struct Grammar;

const SPACE: [char; 6] = [' ', '\t', '\r', '\n', '\u{000B}', '\u{000C}'];
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
        let (position, message) = advice(source)
            .filter(|&(advised, _)| advised <= position)
            .unwrap_or_else(|| (position, error.variant.message().into_owned()));
        let length = source[position..].chars().next().map_or(0, char::len_utf8);
        Failure::Syntax {
            message,
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

fn advice(source: &str) -> Option<(usize, String)> {
    let mut open = Vec::new();
    let mut sink = false;
    let mut previous = None;
    let mut word = false;
    for (position, character) in source.char_indices() {
        if SPACE.contains(&character) {
            word = false;
            continue;
        }
        let current = if DELIMITER.contains(&character) {
            word = false;
            character
        } else if word {
            continue;
        } else {
            word = true;
            'a'
        };
        let start = matches!(current, 'a' | '(') && !matches!(previous, Some('.' | 'a' | ')'));
        let closing = match current {
            ')' => Some('('),
            ']' => Some('['),
            _ => None,
        };
        let message = match (previous, current) {
            _ if closing.is_some() && open.last().map(|&(delimiter, _)| delimiter) != closing => {
                Some("this closes nothing that is open here")
            }
            (Some('a' | ')'), 'a' | '(') => {
                Some("put a dot between these to join them, or a comma to separate them")
            }
            (Some('.'), '[') | (Some(']'), '.') => {
                Some("a rule joins a particle inside parentheses, as in X.([A] B)")
            }
            (Some('.'), '.' | ',' | ')' | ']') | (None | Some(',' | '(' | '['), '.') => {
                Some("a dot joins two things; put something on each side")
            }
            (None | Some(',' | '(' | '['), ',') => {
                Some("a comma separates two things; put something on each side")
            }
            _ if start && sink => Some(
                "put a dot between the two particles to join them, or a comma to separate them",
            ),
            _ => None,
        };
        if let Some(message) = message {
            return Some((position, message.into()));
        }
        sink |= start;
        match current {
            '(' | '[' => open.push((current, std::mem::take(&mut sink))),
            ')' | ']' => sink = open.pop().is_some_and(|(_, sink)| sink),
            ',' => sink = false,
            _ => {}
        }
        previous = Some(current);
    }
    if previous == Some('.') {
        return Some((
            source.len(),
            "a dot joins two things; put something on each side".into(),
        ));
    }
    (!open.is_empty()).then(|| (source.len(), "close what is still open".into()))
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
