use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Code {
    Request,
    Version,
    Size,
    Source,
    Target,
}

#[derive(Serialize)]
pub struct Failure {
    code: Code,
    message: String,
}

impl Failure {
    pub fn new(code: Code, message: impl ToString) -> Self {
        Self {
            code,
            message: message.to_string(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}
