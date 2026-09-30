const DELIMITER: [char; 6] = ['(', ')', '[', ']', '.', ','];

// Editors on Windows begin a file with U+FEFF, the byte order mark, so it reads as the space it
// once named beside Unicode's pattern whitespace.
pub fn space(character: char) -> bool {
    pest::unicode::PATTERN_WHITE_SPACE(character) || character == '\u{FEFF}'
}

pub fn delimiter(character: char) -> bool {
    DELIMITER.contains(&character)
}

pub fn separator(character: char) -> bool {
    space(character) || delimiter(character)
}

// Spaces other than whitespace would make an atom look like two joined ones, invisible characters
// would make atoms that look alike differ, direction controls reorder how a line reads, and control
// characters act on the terminal that prints them. The joiners that emoji sequences and some
// scripts need stay.
pub fn refused(character: char) -> bool {
    !space(character)
        && (pest::unicode::SPACE_SEPARATOR(character)
            || pest::unicode::CONTROL(character)
            || pest::unicode::BIDI_CONTROL(character)
            || matches!(character, '\u{200B}' | '\u{2060}'))
}

pub fn point(character: char) -> String {
    format!("U+{:04X}", u32::from(character))
}
