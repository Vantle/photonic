use crate::character;

const DOT: &str = "a dot joins two things; put something on each side";

struct Opener {
    delimiter: char,
    position: usize,
    sink: bool,
}

// The first place a reader should look, with what to write there. Pest reports where parsing
// stopped, which for a forgotten comma or an unclosed group is often far from the mistake.
pub(crate) fn find(source: &str) -> Option<(usize, String)> {
    let mut open = Vec::<Opener>::new();
    let mut sink = false;
    let mut previous = None;
    let mut word = false;
    for (position, character) in source.char_indices() {
        if character::space(character) {
            word = false;
            continue;
        }
        if character::refused(character) {
            return Some((
                position,
                format!("{} cannot appear in an atom", character::point(character)),
            ));
        }
        let current = if character::delimiter(character) {
            word = false;
            character
        } else if word {
            continue;
        } else {
            word = true;
            'a'
        };
        if let Some(message) = close(source, current, open.last()) {
            return Some((position, message));
        }
        let start = matches!(current, 'a' | '(') && !matches!(previous, Some('.' | 'a' | ')'));
        let message = match (previous, current) {
            (Some('a' | ')'), 'a' | '(') => {
                Some("put a dot between these to join them, or a comma to separate them")
            }
            (Some('.'), '[') | (Some(']'), '.') => {
                Some("a rule joins a particle inside parentheses, as in X.([A] B)")
            }
            (Some('.'), '.' | ',' | ')' | ']') | (None | Some(',' | '(' | '['), '.') => Some(DOT),
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
            '(' | '[' => open.push(Opener {
                delimiter: current,
                position,
                sink: std::mem::take(&mut sink),
            }),
            ')' | ']' => sink = open.pop().is_some_and(|opener| opener.sink),
            ',' => sink = false,
            _ => {}
        }
        previous = Some(current);
    }
    if previous == Some('.') {
        return Some((source.len(), DOT.into()));
    }
    open.last().map(|opener| {
        (
            opener.position,
            format!("this {} is never closed", opener.delimiter),
        )
    })
}

fn close(source: &str, current: char, innermost: Option<&Opener>) -> Option<String> {
    let expected = match current {
        ')' => '(',
        ']' => '[',
        _ => return None,
    };
    let Some(opener) = innermost else {
        return Some(format!("this {current} closes nothing that is open"));
    };
    (opener.delimiter != expected).then(|| {
        format!(
            "this {current} does not close the {} at {}",
            opener.delimiter,
            place(source, opener.position)
        )
    })
}

fn place(source: &str, position: usize) -> String {
    let prefix = &source[..position];
    let start = prefix.rfind('\n').map_or(0, |index| index + 1);
    format!(
        "{}:{}",
        prefix.matches('\n').count() + 1,
        prefix[start..].chars().count() + 1
    )
}
