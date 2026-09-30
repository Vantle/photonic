use serde::Deserialize;

#[derive(Deserialize)]
pub struct Case {
    pub name: String,
    pub program: frontend::source::Program,
    pub closed: bool,
}

pub fn load() -> Result<Vec<Case>, serde_json::Error> {
    serde_json::from_str(include_str!("../language/test/reference.json"))
}
