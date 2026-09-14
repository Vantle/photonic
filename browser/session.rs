use crate::expression;
use photonic::path::Search;
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

#[wasm_bindgen]
pub struct Evaluation {
    search: Search,
    source: String,
}

#[wasm_bindgen]
impl Evaluation {
    #[wasm_bindgen(constructor)]
    pub fn new(input: &str) -> Result<Evaluation, JsValue> {
        let (search, source) = expression::prepare(input)
            .map_err(|error| JsValue::from_str(&serde_json::to_string(&error).unwrap()))?;
        Ok(Self { search, source })
    }

    pub fn run(&mut self) -> String {
        expression::advance(&mut self.search);
        let summary = self.search.summary();
        serde_json::json!({"state": self.search.current(), "source": self.source, "work": summary.work, "event": summary.event}).to_string()
    }

    pub fn inspect(&self, index: usize) -> String {
        let Some(event) = self.search.transition(index) else {
            return serde_json::json!({"error": "No such transition."}).to_string();
        };
        serde_json::json!({"index": index, "event": event, "before": self.search.inspect(event.source), "after": self.search.inspect(event.target)}).to_string()
    }
}
