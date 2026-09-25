#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum Failure {
    #[error("base must be between {minimum} and {maximum}, received {value}")]
    Base {
        value: u32,
        minimum: u32,
        maximum: u32,
    },
    #[error("width must be between {minimum} and {maximum}, received {value}")]
    Width {
        value: usize,
        minimum: usize,
        maximum: usize,
    },
    #[error("the value does not fit the selected representation")]
    Capacity,
    #[error(
        "expected decimal digits or a 0b, 0t, 0o, or 0x prefix; underscores may separate digits"
    )]
    Syntax,
    #[error("the numeral exceeds the supported integer range")]
    Range,
    #[error("this operand must be nonnegative")]
    Sign,
}
