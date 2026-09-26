use miette::Diagnostic;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Code {
    Request,
    Version,
    Size,
    Budget,
    Source,
    Library,
    Target,
    Internal,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Item {
    Program(usize),
    Target(usize),
}

#[derive(Serialize)]
pub struct Span {
    offset: usize,
    length: usize,
}

#[derive(Serialize)]
pub struct Failure {
    code: Code,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<Span>,
    #[serde(flatten)]
    item: Option<Item>,
}

fn character(source: &str, byte: usize) -> usize {
    source
        .get(..byte)
        .map_or(byte, |prefix| prefix.encode_utf16().count())
}

impl Failure {
    pub fn new(code: Code, message: impl ToString) -> Self {
        Self {
            code,
            message: message.to_string(),
            span: None,
            item: None,
        }
    }

    pub fn within(self, item: Item) -> Self {
        Self {
            item: Some(item),
            ..self
        }
    }

    pub fn located(code: Code, error: &(impl Diagnostic + ToString), source: &str) -> Self {
        let span = error
            .labels()
            .and_then(|mut label| label.next())
            .map(|label| {
                let offset = character(source, label.offset());
                Span {
                    offset,
                    length: character(source, label.offset() + label.len()) - offset,
                }
            });
        Self {
            span,
            ..Self::new(code, error.to_string())
        }
    }
}

impl From<translation::failure::Failure> for Failure {
    fn from(error: translation::failure::Failure) -> Self {
        match error {
            translation::failure::Failure::Vocabulary { .. } => Self::new(Code::Size, error),
            translation::failure::Failure::Duplicate { .. }
            | translation::failure::Failure::Name { .. } => Self::new(Code::Source, error),
        }
    }
}
