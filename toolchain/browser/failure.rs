use miette::Diagnostic;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Code {
    Request,
    Version,
    Size,
    Source,
    Library,
    Target,
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
}

fn unit(source: &str, offset: usize) -> usize {
    source
        .get(..offset)
        .map_or(offset, |prefix| prefix.encode_utf16().count())
}

impl Failure {
    pub fn new(code: Code, message: impl ToString) -> Self {
        Self {
            code,
            message: message.to_string(),
            span: None,
        }
    }

    pub fn located(code: Code, error: &(impl Diagnostic + ToString), source: &str) -> Self {
        let span = error
            .labels()
            .and_then(|mut label| label.next())
            .map(|label| {
                let offset = unit(source, label.offset());
                Span {
                    offset,
                    length: unit(source, label.offset() + label.len()) - offset,
                }
            });
        Self {
            code,
            message: error.to_string(),
            span,
        }
    }
}
