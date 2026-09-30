use crate::character;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refusal {
    Empty,
    Character(char),
}

pub fn check(text: &str) -> Result<(), Refusal> {
    if text.is_empty() {
        return Err(Refusal::Empty);
    }
    text.chars()
        .find(|&character| character::separator(character) || character::refused(character))
        .map_or(Ok(()), |character| Err(Refusal::Character(character)))
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Empty => formatter.write_str("an atom holds at least one character"),
            Self::Character(character) if character::delimiter(character) => {
                write!(
                    formatter,
                    "'{character}' separates atoms, so an atom cannot hold it"
                )
            }
            Self::Character(character) if character::space(character) => write!(
                formatter,
                "{} is a space, which separates atoms, so an atom cannot hold it",
                character::point(character)
            ),
            Self::Character(character) => write!(
                formatter,
                "{} cannot appear in an atom",
                character::point(character)
            ),
        }
    }
}
