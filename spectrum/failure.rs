use schemars::JsonSchema;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Code {
    Request,
    File,
    Source,
    Library,
    Target,
    Pattern,
    Handle,
    Exploration,
    Claim,
    Shape,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Location {
    pub(crate) file: String,
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) length: usize,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Failure {
    pub code: Code,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) location: Option<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) diagnostic: Option<String>,
}

impl Location {
    pub(crate) fn new(file: &str, text: &str, offset: usize, length: usize) -> Self {
        let offset = offset.min(text.len());
        let prefix = text.get(..offset).unwrap_or_default();
        let start = prefix.rfind('\n').map_or(0, |index| index + 1);
        let end = (offset + length).min(text.len());
        Self {
            file: file.to_owned(),
            line: prefix.matches('\n').count() + 1,
            column: prefix[start..].chars().count() + 1,
            length: text.get(offset..end).map_or(0, |span| span.chars().count()),
        }
    }
}

impl Failure {
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            location: None,
            diagnostic: None,
        }
    }

    pub(crate) fn located(
        code: Code,
        error: &(impl miette::Diagnostic + ToString),
        file: &str,
        text: &str,
    ) -> Self {
        let location = error
            .labels()
            .and_then(|mut label| label.next())
            .map(|label| Location::new(file, text, label.offset(), label.len()));
        let diagnostic = error.code().map(|code| {
            let code = code.to_string();
            code.rsplit("::").next().unwrap_or_default().to_owned()
        });
        Self {
            code,
            message: error.to_string(),
            location,
            diagnostic,
        }
    }
}

impl std::fmt::Display for Code {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&crate::render::name(self))
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.location {
            Some(location) => write!(
                formatter,
                "{}:{}:{}: {}",
                location.file, location.line, location.column, self.message
            ),
            None => write!(formatter, "{}", self.message),
        }
    }
}

impl std::error::Error for Failure {}
