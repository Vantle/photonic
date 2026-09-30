use crate::character;

// A text fails to parse only where a bracket is unbalanced or an atom holds a refused character,
// and pest reports an unclosed bracket where the text ends, far from the bracket itself.
pub(crate) fn find(source: &str) -> Option<(usize, String)> {
    let mut open = Vec::<(char, usize)>::new();
    for (position, character) in source.char_indices() {
        if character::refused(character) {
            return Some((
                position,
                format!("{} cannot appear in an atom", character::point(character)),
            ));
        }
        let expected = match character {
            '(' | '[' => {
                open.push((character, position));
                continue;
            }
            ')' => '(',
            ']' => '[',
            _ => continue,
        };
        match open.pop() {
            None => {
                return Some((
                    position,
                    format!("this {character} closes nothing that is open"),
                ));
            }
            Some((opener, place)) if opener != expected => {
                return Some((
                    position,
                    format!(
                        "this {character} does not close the {opener} at {}",
                        location(source, place)
                    ),
                ));
            }
            Some(_) => {}
        }
    }
    open.last()
        .map(|&(opener, position)| (position, format!("this {opener} is never closed")))
}

fn location(source: &str, position: usize) -> String {
    let prefix = &source[..position];
    let start = prefix.rfind('\n').map_or(0, |index| index + 1);
    format!(
        "{}:{}",
        prefix.matches('\n').count() + 1,
        prefix[start..].chars().count() + 1
    )
}
