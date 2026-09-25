use crate::verb::Disk;
use miette::IntoDiagnostic;
use serde_json::{Map, Value, json};
use spectrum::context::Context;
use spectrum::request::{self, Request};
use spectrum::store::Store;
use std::io::{BufRead, Write};
use std::process::ExitCode;

const MODERN: &str = "2026-07-28";
const LEGACY: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
const STRUCTURED: &str = "2025-06-18";
const VERSION: &str = "io.modelcontextprotocol/protocolVersion";
const CAPABILITY: &str = "io.modelcontextprotocol/clientCapabilities";
const SERVER: &str = "io.modelcontextprotocol/serverInfo";
const INSTRUCTION: &str = "Spectrum answers questions about Photonic programs and every future they can reach. Start with check or explore on the program's files. Answers name rules, configurations, events and occurrences by handles such as r2, s11, e12 and s11.o1; pass them to inspect, cause, miss and step. Every answer about an exploration carries its key, such as x91c7f661; pass it as exploration instead of the program to ask more of the same recording. Claims answer holds, fails or unknown; unknown means the search could not settle the claim, because a budget stopped it or a direct path follows one run, and is never evidence of absence. Read photonic://primer for the language.";

#[derive(Clone, Copy, Eq, PartialEq)]
enum Era {
    Modern,
    Legacy(&'static str),
}

pub struct Server {
    reader: Disk,
    store: Store,
    legacy: Option<&'static str>,
}

fn information() -> Value {
    json!({
        "name": "photonic",
        "title": "Photonic Spectrum",
        "version": env!("CARGO_PKG_VERSION"),
    })
}

fn capability() -> Value {
    json!({ "tools": {}, "resources": {} })
}

fn failure(code: i64, message: &str, data: Option<Value>) -> Value {
    let mut error = json!({ "code": code, "message": message });
    if let (Some(data), Some(object)) = (data, error.as_object_mut()) {
        object.insert("data".to_owned(), data);
    }
    error
}

fn supported() -> Vec<&'static str> {
    std::iter::once(MODERN).chain(LEGACY).collect()
}

fn structured(era: Era) -> bool {
    match era {
        Era::Modern => true,
        Era::Legacy(version) => version >= STRUCTURED,
    }
}

impl Server {
    pub fn new() -> Self {
        Self {
            reader: Disk,
            store: Store::new(64),
            legacy: None,
        }
    }

    fn era(&self, parameter: &Map<String, Value>) -> Result<Era, Value> {
        let meta = parameter.get("_meta").and_then(Value::as_object);
        let Some(requested) = meta.and_then(|meta| meta.get(VERSION)) else {
            return Ok(Era::Legacy(self.legacy.unwrap_or(LEGACY[0])));
        };
        let requested = requested.as_str().unwrap_or_default();
        if let Some(version) = LEGACY.into_iter().find(|version| *version == requested) {
            return Ok(Era::Legacy(version));
        }
        if requested != MODERN {
            return Err(failure(
                -32022,
                "Unsupported protocol version",
                Some(json!({ "supported": supported(), "requested": requested })),
            ));
        }
        if meta.and_then(|meta| meta.get(CAPABILITY)).is_none() {
            return Err(failure(
                -32602,
                "Invalid params: _meta must carry io.modelcontextprotocol/clientCapabilities",
                None,
            ));
        }
        Ok(Era::Modern)
    }

    fn complete(era: Era, mut result: Map<String, Value>) -> Value {
        if era == Era::Modern {
            result.insert("resultType".to_owned(), json!("complete"));
            let mut meta = result
                .remove("_meta")
                .and_then(|meta| meta.as_object().cloned())
                .unwrap_or_default();
            meta.insert(SERVER.to_owned(), information());
            result.insert("_meta".to_owned(), Value::Object(meta));
        }
        Value::Object(result)
    }

    fn initialize(&mut self, parameter: &Map<String, Value>) -> Value {
        let requested = parameter
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let version = LEGACY
            .into_iter()
            .find(|version| *version == requested)
            .unwrap_or(LEGACY[0]);
        self.legacy = Some(version);
        json!({
            "protocolVersion": version,
            "capabilities": capability(),
            "serverInfo": information(),
            "instructions": INSTRUCTION,
        })
    }

    fn discover() -> Map<String, Value> {
        let mut result = Map::new();
        result.insert("supportedVersions".to_owned(), json!(supported()));
        result.insert("capabilities".to_owned(), capability());
        result.insert("instructions".to_owned(), json!(INSTRUCTION));
        result
    }

    fn tool(era: Era) -> Map<String, Value> {
        let tool = request::catalog()
            .iter()
            .map(|tool| {
                let mut entry = json!({
                    "name": tool.name,
                    "title": tool.title,
                    "description": tool.description,
                    "inputSchema": tool.input,
                });
                if structured(era)
                    && let Some(object) = entry.as_object_mut()
                {
                    object.insert("outputSchema".to_owned(), tool.output.clone());
                }
                entry
            })
            .collect::<Vec<_>>();
        let mut result = Map::new();
        result.insert("tools".to_owned(), json!(tool));
        result
    }

    fn call(
        &mut self,
        era: Era,
        parameter: &Map<String, Value>,
    ) -> Result<Map<String, Value>, Value> {
        let name = parameter
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !request::catalog().iter().any(|tool| tool.name == name) {
            return Err(failure(-32602, &format!("Unknown tool: {name}"), None));
        }
        let argument = parameter
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let mut context = Context {
            reader: &self.reader,
            store: &mut self.store,
        };
        let answer = Request::read(name, argument).and_then(|request| request.answer(&mut context));
        let mut result = Map::new();
        match answer {
            Ok(answer) => {
                result.insert(
                    "content".to_owned(),
                    json!([{ "type": "text", "text": answer.text() }]),
                );
                if structured(era) {
                    result.insert(
                        "structuredContent".to_owned(),
                        serde_json::to_value(&answer).unwrap_or(Value::Null),
                    );
                }
                result.insert("isError".to_owned(), json!(false));
            }
            Err(failure) => {
                result.insert(
                    "content".to_owned(),
                    json!([{ "type": "text", "text": failure.to_string() }]),
                );
                result.insert("isError".to_owned(), json!(true));
            }
        }
        Ok(result)
    }

    fn resource() -> Map<String, Value> {
        let entry = spectrum::resource::catalog()
            .iter()
            .map(|resource| {
                json!({
                    "uri": resource.uri,
                    "name": resource.name,
                    "title": resource.title,
                    "description": resource.description,
                    "mimeType": resource.mime,
                    "size": resource.text.len(),
                })
            })
            .collect::<Vec<_>>();
        let mut result = Map::new();
        result.insert("resources".to_owned(), json!(entry));
        result
    }

    fn read(era: Era, parameter: &Map<String, Value>) -> Result<Map<String, Value>, Value> {
        let uri = parameter
            .get("uri")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(resource) = spectrum::resource::find(uri) else {
            let code = if era == Era::Modern { -32602 } else { -32002 };
            return Err(failure(
                code,
                "Resource not found",
                Some(json!({ "uri": uri })),
            ));
        };
        let mut result = Map::new();
        result.insert(
            "contents".to_owned(),
            json!([{ "uri": resource.uri, "mimeType": resource.mime, "text": resource.text }]),
        );
        Ok(result)
    }

    fn dispatch(&mut self, method: &str, parameter: &Map<String, Value>) -> Result<Value, Value> {
        if method == "initialize" {
            return Ok(self.initialize(parameter));
        }
        let era = self.era(parameter)?;
        let result = match method {
            "server/discover" => Self::discover(),
            "ping" => Map::new(),
            "tools/list" => Self::tool(era),
            "tools/call" => self.call(era, parameter)?,
            "resources/list" => Self::resource(),
            "resources/templates/list" => {
                let mut result = Map::new();
                result.insert("resourceTemplates".to_owned(), json!([]));
                result
            }
            "resources/read" => Self::read(era, parameter)?,
            _ => {
                return Err(failure(
                    -32601,
                    &format!("Method not found: {method}"),
                    None,
                ));
            }
        };
        Ok(Self::complete(era, result))
    }

    fn message(&mut self, message: Value) -> Option<Value> {
        let Value::Object(message) = message else {
            let invalid = failure(-32600, "Invalid Request: send a JSON object", None);
            return Some(reply(Value::Null, Err(invalid)));
        };
        let id = message.get("id").cloned();
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            if message.contains_key("result") || message.contains_key("error") {
                return None;
            }
            let invalid = failure(-32600, "Invalid Request: name the method", None);
            return Some(reply(id.unwrap_or(Value::Null), Err(invalid)));
        };
        let parameter = message
            .get("params")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let result = self.dispatch(method, &parameter);
        Some(reply(id?, result))
    }

    pub fn handle(&mut self, line: &str) -> Option<String> {
        let message = match serde_json::from_str::<Value>(line) {
            Ok(message) => message,
            Err(error) => {
                let parse = failure(-32700, &format!("Parse error: {error}"), None);
                return Some(reply(Value::Null, Err(parse)).to_string());
            }
        };
        let Value::Array(batch) = message else {
            return self.message(message).map(|response| response.to_string());
        };
        if batch.is_empty() {
            let empty = failure(-32600, "Invalid Request: the batch is empty", None);
            return Some(reply(Value::Null, Err(empty)).to_string());
        }
        let response = batch
            .into_iter()
            .filter_map(|message| self.message(message))
            .collect::<Vec<_>>();
        (!response.is_empty()).then(|| Value::Array(response).to_string())
    }
}

fn reply(id: Value, result: Result<Value, Value>) -> Value {
    match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(error) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
    }
}

pub fn serve() -> miette::Result<ExitCode> {
    let mut server = Server::new();
    let input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    for line in input.lines() {
        let line = line.into_diagnostic()?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = server.handle(&line) {
            writeln!(output, "{response}").into_diagnostic()?;
            output.flush().into_diagnostic()?;
        }
    }
    Ok(ExitCode::SUCCESS)
}
