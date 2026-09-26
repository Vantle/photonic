use crate::context::Context;
use crate::failure::{Code, Failure};
use crate::{cause, check, compare, explore, inspect, miss, select, shape, step};
use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;

const VERSION: u64 = 1;

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(tag = "verb", rename_all = "lowercase")]
pub enum Request {
    Check(check::Request),
    Explore(explore::Request),
    Select(select::Request),
    Inspect(inspect::Request),
    Cause(cause::Request),
    Miss(miss::Request),
    Step(step::Request),
    Compare(compare::Request),
    Shape(shape::Request),
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Answer {
    Check(check::Answer),
    Explore(explore::Answer),
    Select(select::Answer),
    Inspect(inspect::Answer),
    Cause(cause::Answer),
    Miss(miss::Answer),
    Step(step::Answer),
    Compare(compare::Answer),
    Shape(shape::Answer),
}

pub struct Tool {
    pub name: &'static str,
    pub title: &'static str,
    pub description: String,
    pub input: Value,
    pub output: Value,
}

#[derive(Serialize)]
struct Envelope<'value> {
    version: u64,
    verb: &'value str,
    #[serde(skip_serializing_if = "Option::is_none")]
    answer: Option<&'value Answer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'value Failure>,
}

impl Request {
    pub fn verb(&self) -> &'static str {
        match self {
            Self::Check(_) => "check",
            Self::Explore(_) => "explore",
            Self::Select(_) => "select",
            Self::Inspect(_) => "inspect",
            Self::Cause(_) => "cause",
            Self::Miss(_) => "miss",
            Self::Step(_) => "step",
            Self::Compare(_) => "compare",
            Self::Shape(_) => "shape",
        }
    }

    pub fn answer(&self, context: &mut Context<'_>) -> Result<Answer, Failure> {
        Ok(match self {
            Self::Check(request) => Answer::Check(check::answer(request, context)?),
            Self::Explore(request) => Answer::Explore(explore::answer(request, context)?),
            Self::Select(request) => Answer::Select(select::answer(request, context)?),
            Self::Inspect(request) => Answer::Inspect(inspect::answer(request, context)?),
            Self::Cause(request) => Answer::Cause(cause::answer(request, context)?),
            Self::Miss(request) => Answer::Miss(miss::answer(request, context)?),
            Self::Step(request) => Answer::Step(step::answer(request, context)?),
            Self::Compare(request) => Answer::Compare(compare::answer(request, context)?),
            Self::Shape(request) => Answer::Shape(shape::answer(request, context)?),
        })
    }

    pub fn read(verb: &str, argument: Value) -> Result<Self, Failure> {
        let Value::Object(mut object) = argument else {
            return Err(Failure::new(
                Code::Request,
                "the arguments must be an object",
            ));
        };
        let tool = catalog()
            .iter()
            .find(|tool| tool.name == verb)
            .ok_or_else(|| {
                Failure::new(
                    Code::Request,
                    format!(
                        "{verb} is not a verb; the verbs are {}",
                        list(catalog().iter().map(|tool| tool.name))
                    ),
                )
            })?;
        let argument = Value::Object(object.clone());
        if let Some(message) = unknown(&argument, &tool.input, "") {
            return Err(Failure::new(Code::Request, message));
        }
        object.insert("verb".to_owned(), Value::String(verb.to_owned()));
        serde_json::from_value(Value::Object(object))
            .map_err(|error| Failure::new(Code::Request, error.to_string()))
    }
}

impl Answer {
    pub fn passed(&self) -> bool {
        match self {
            Self::Check(answer) => answer.passed(),
            Self::Compare(answer) => answer.passed(),
            Self::Shape(answer) => answer.passed(),
            _ => true,
        }
    }

    pub fn text(&self) -> String {
        match self {
            Self::Check(answer) => answer.text(),
            Self::Explore(answer) => answer.text(),
            Self::Select(answer) => answer.text(),
            Self::Inspect(answer) => answer.text(),
            Self::Cause(answer) => answer.text(),
            Self::Miss(answer) => answer.text(),
            Self::Step(answer) => answer.text(),
            Self::Compare(answer) => answer.text(),
            Self::Shape(answer) => answer.text(),
        }
    }
}

pub fn envelope(verb: &str, result: &Result<Answer, Failure>) -> String {
    let envelope = match result {
        Ok(answer) => Envelope {
            version: VERSION,
            verb,
            answer: Some(answer),
            error: None,
        },
        Err(failure) => Envelope {
            version: VERSION,
            verb,
            answer: None,
            error: Some(failure),
        },
    };
    serde_json::to_string(&envelope).unwrap_or_default()
}

fn list<'name>(name: impl Iterator<Item = &'name str>) -> String {
    name.collect::<Vec<_>>().join(", ")
}

fn branch(schema: &Value) -> impl Iterator<Item = &Value> {
    ["anyOf", "oneOf", "allOf"]
        .into_iter()
        .filter_map(|key| schema.get(key).and_then(Value::as_array))
        .flatten()
}

fn unknown(value: &Value, schema: &Value, path: &str) -> Option<String> {
    match value {
        Value::Object(object) => {
            let Some(property) = schema.get("properties").and_then(Value::as_object) else {
                return branch(schema)
                    .filter(|branch| branch.get("properties").is_some())
                    .find_map(|branch| unknown(value, branch, path));
            };
            object
                .iter()
                .find_map(|(key, inner)| match property.get(key) {
                    Some(schema) => unknown(inner, schema, &format!("{path}{key}.")),
                    None => Some(format!(
                        "{path}{key} is not a field here; the fields are {}",
                        list(property.keys().map(String::as_str))
                    )),
                })
        }
        Value::Array(item) => {
            let schema = schema.get("items")?;
            item.iter()
                .enumerate()
                .find_map(|(position, item)| unknown(item, schema, &format!("{path}{position}.")))
        }
        _ => None,
    }
}

fn close(schema: &mut Value) {
    let Value::Object(object) = schema else {
        return;
    };
    if object.contains_key("properties") {
        object.insert("additionalProperties".to_owned(), Value::Bool(false));
    }
    for (key, inner) in object.iter_mut() {
        match key.as_str() {
            "properties" => inner
                .as_object_mut()
                .into_iter()
                .flat_map(|property| property.values_mut())
                .for_each(close),
            "items" => close(inner),
            "anyOf" | "oneOf" | "allOf" => {
                inner.as_array_mut().into_iter().flatten().for_each(close)
            }
            _ => {}
        }
    }
}

fn schema<Type: JsonSchema>(settings: SchemaSettings) -> Value {
    settings
        .with(|settings| settings.inline_subschemas = true)
        .into_generator()
        .into_root_schema_for::<Type>()
        .to_value()
}

fn tool<Input: JsonSchema, Output: JsonSchema>(name: &'static str, title: &'static str) -> Tool {
    let mut input = schema::<Input>(SchemaSettings::draft2020_12());
    close(&mut input);
    let description = input
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    Tool {
        name,
        title,
        description,
        input,
        output: schema::<Output>(SchemaSettings::draft2020_12().for_serialize()),
    }
}

pub fn catalog() -> &'static [Tool] {
    static CATALOG: OnceLock<Vec<Tool>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        vec![
            tool::<check::Request, check::Answer>("check", "Check a program"),
            tool::<explore::Request, explore::Answer>("explore", "Explore every future"),
            tool::<select::Request, select::Answer>("select", "Select by pattern"),
            tool::<inspect::Request, inspect::Answer>("inspect", "Inspect a handle"),
            tool::<cause::Request, cause::Answer>("cause", "Explain why something is here"),
            tool::<miss::Request, miss::Answer>("miss", "Explain why not"),
            tool::<step::Request, step::Answer>("step", "List the agenda"),
            tool::<compare::Request, compare::Answer>("compare", "Compare two programs"),
            tool::<shape::Request, shape::Answer>("shape", "Describe the shape"),
        ]
    })
}
