use crate::failure::Failure;
use serde::Serialize;

#[derive(Serialize)]
struct Envelope<Body> {
    version: u32,
    #[serde(flatten)]
    body: Body,
}

#[derive(Serialize)]
struct Rejection {
    error: Failure,
}

pub fn respond(result: Result<impl Serialize, Failure>) -> String {
    match result {
        Ok(body) => encode(body),
        Err(error) => encode(Rejection { error }),
    }
}

pub fn encode(body: impl Serialize) -> String {
    serde_json::to_string(&Envelope { version: 1, body }).unwrap()
}
