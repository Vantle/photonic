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

pub fn parse(source: &str) -> Result<Tree<'_>, Failure> {
    depth(source)?;
    let parsed = Grammar::parse(Rule::module, source).map_err(|error| {
        let (position, message) = advice(source).unwrap_or_else(|| {
            let position = match error.location {
                InputLocation::Pos(position) | InputLocation::Span((position, _)) => position,
            };
            (position, error.variant.message().into_owned())
        });
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
    Ok(Tree { source, node })
}

fn advice(source: &str) -> Option<(usize, String)> {
    let space = [' ', '\t', '\r', '\n', '\u{000B}', '\u{000C}'];
    let mut open = Vec::new();
    let mut sink = false;
    let mut previous = None;
    let mut word = false;
    for (position, character) in source.char_indices() {
        if space.contains(&character) {
            word = false;
            continue;
        }
        let current = if "()[].,".contains(character) {
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
    (!open.is_empty()).then(|| (source.len(), "close what is still open".into()))
}

pub const DEPTH: usize = 128;

fn depth(source: &str) -> Result<(), Failure> {
    let bracket = bracket(source);
    let mut stack = vec![0];
    let mut next = 1;
    let mut level = 0;
    let enter = |term: usize, level: &mut usize| {
        let base = *level;
        *level += bracket[term].len();
        if *level > DEPTH {
            return Err(Failure::Depth {
                limit: DEPTH,
                span: (bracket[term][DEPTH - base], 1).into(),
            });
        }
        Ok(())
    };
    enter(0, &mut level)?;
    for (position, byte) in source.bytes().enumerate() {
        match byte {
            b'(' | b'[' => {
                if byte == b'(' {
                    level += 1;
                    if level > DEPTH {
                        return Err(Failure::Depth {
                            limit: DEPTH,
                            span: (position, 1).into(),
                        });
                    }
                }
                stack.push(next);
                enter(next, &mut level)?;
                next += 1;
            }
            b')' | b']' if stack.len() > 1 => {
                let term = stack.pop().expect("a term below the root is open");
                level -= bracket[term].len() + usize::from(byte == b')');
            }
            b',' => {
                let term = stack.last_mut().expect("the root term stays open");
                level -= bracket[*term].len();
                *term = next;
                enter(next, &mut level)?;
                next += 1;
            }
            _ => {}
        }
    }
    Ok(())
}

fn bracket(source: &str) -> Vec<Vec<usize>> {
    let mut bracket = vec![Vec::new()];
    let mut stack = vec![0];
    for (position, byte) in source.bytes().enumerate() {
        match byte {
            b'(' | b'[' => {
                if byte == b'[' {
                    bracket[stack[stack.len() - 1]].push(position);
                }
                stack.push(bracket.len());
                bracket.push(Vec::new());
            }
            b')' | b']' if stack.len() > 1 => {
                stack.pop();
            }
            b',' => {
                let last = stack.len() - 1;
                stack[last] = bracket.len();
                bracket.push(Vec::new());
            }
            _ => {}
        }
    }
    bracket
}
