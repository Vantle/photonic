use crate::expression::{self, Calculation};
use crate::failure::{Code, Failure};
use crate::response;
use photonic::path::{Event, Search};
use photonic::snapshot::{Definition, Node};
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, wasm_bindgen};

#[derive(Serialize)]
struct Inspection<'search> {
    definition: Vec<Definition>,
    index: usize,
    event: &'search Event,
    before: Option<Node>,
    after: Option<Node>,
}

#[wasm_bindgen]
pub struct Evaluation {
    search: Search,
    source: String,
}

#[wasm_bindgen]
impl Evaluation {
    #[wasm_bindgen(constructor)]
    pub fn new(input: &str) -> Result<Self, JsError> {
        let (search, source) =
            expression::prepare(input).map_err(|failure| JsError::new(failure.message()))?;
        Ok(Self { search, source })
    }

    pub fn run(&mut self) -> String {
        expression::advance(&mut self.search);
        response::encode(Calculation::new(&self.search, self.source.clone()))
    }

    pub fn inspect(&self, index: usize) -> String {
        response::respond(
            self.search
                .transition(index)
                .map(|event| Inspection {
                    definition: self.search.definition(),
                    index,
                    event,
                    before: self.search.inspect(event.source),
                    after: self.search.inspect(event.target),
                })
                .ok_or_else(|| Failure::new(Code::Request, "No such transition.")),
        )
    }
}
