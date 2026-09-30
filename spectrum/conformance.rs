use crate::failure::{Code, Failure};
use crate::request;
use serde_json::{Map, Value};

// Whether a request's arguments fit the input schema its verb publishes, checked before the request
// is read, so that a field of the wrong type or a required field that is missing is named by its
// path, as the reader names an unknown field, rather than by the type the reader expected. What the
// schema leaves to the reader, such as an unknown verb or field, passes here.
pub fn verify(verb: &str, argument: &Value) -> Result<(), Failure> {
    let Some(tool) = request::catalog().iter().find(|tool| tool.name == verb) else {
        return Ok(());
    };
    let Value::Object(object) = argument else {
        return Ok(());
    };
    match field(object, &tool.input, "") {
        Some(message) => Err(Failure::new(Code::Request, message)),
        None => Ok(()),
    }
}

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(number) if number.is_u64() || number.is_i64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn noun(kind: &str) -> String {
    match kind {
        "null" => "null".to_owned(),
        "array" | "integer" | "object" => format!("an {kind}"),
        kind => format!("a {kind}"),
    }
}

fn list(word: &[String]) -> String {
    match word {
        [] => String::new(),
        [single] => single.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

fn admit(schema: &Value) -> Vec<&str> {
    match schema.get("type") {
        Some(Value::String(kind)) => vec![kind.as_str()],
        Some(Value::Array(kind)) => kind.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    }
}

// The first way a value departs from its schema, named by the value's path. A value that fits no
// branch of a choice is explained by the branch that takes its type, as the reader would read it.
fn mismatch(value: &Value, schema: &Value, name: &str) -> Option<String> {
    let branch = ["anyOf", "oneOf"]
        .into_iter()
        .filter_map(|key| schema.get(key).and_then(Value::as_array))
        .flatten()
        .collect::<Vec<_>>();
    if let Some(&first) = branch.first()
        && branch
            .iter()
            .all(|branch| mismatch(value, branch, name).is_some())
    {
        let fitting = branch
            .iter()
            .find(|branch| admit(branch).contains(&kind(value)))
            .unwrap_or(&first);
        return mismatch(value, fitting, name);
    }
    let allowed = admit(schema);
    let actual = kind(value);
    if !allowed.is_empty()
        && !allowed
            .iter()
            .any(|&allowed| allowed == actual || (allowed == "number" && actual == "integer"))
    {
        let expected = allowed.iter().map(|kind| noun(kind)).collect::<Vec<_>>();
        return Some(format!(
            "{name} takes {}, not {}",
            list(&expected),
            noun(actual)
        ));
    }
    if let Some(allowed) = schema.get("enum").and_then(Value::as_array)
        && !allowed.contains(value)
    {
        let word = allowed
            .iter()
            .filter(|allowed| !allowed.is_null())
            .map(|allowed| {
                allowed
                    .as_str()
                    .map_or_else(|| allowed.to_string(), str::to_owned)
            })
            .collect::<Vec<_>>();
        return Some(format!("{name} takes {}, not {value}", list(&word)));
    }
    if let (Some(minimum), Some(number)) = (schema.get("minimum"), value.as_f64())
        && minimum.as_f64().is_some_and(|minimum| number < minimum)
    {
        return Some(format!("{name} takes at least {minimum}, not {value}"));
    }
    match value {
        Value::Object(object) => field(object, schema, &format!("{name}.")),
        Value::Array(item) => {
            let schema = schema.get("items")?;
            item.iter()
                .enumerate()
                .find_map(|(position, item)| mismatch(item, schema, &format!("{name}.{position}")))
        }
        _ => None,
    }
}

fn field(object: &Map<String, Value>, schema: &Value, prefix: &str) -> Option<String> {
    let missing = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .find(|key| !object.contains_key(*key));
    if let Some(key) = missing {
        return Some(format!("{prefix}{key} is required"));
    }
    let property = schema.get("properties").and_then(Value::as_object)?;
    object.iter().find_map(|(key, inner)| {
        property
            .get(key)
            .and_then(|schema| mismatch(inner, schema, &format!("{prefix}{key}")))
    })
}
