use crate::source::{Definition, Output, Value};

pub fn rule(input: &[Vec<Value>], output: &[Output]) -> String {
    std::iter::once(pattern(input))
        .chain(product(output))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn definition(value: &Definition) -> String {
    if !value.name.is_empty() {
        return value.name.clone();
    }
    rule(&value.input, &value.output)
}

fn pattern(input: &[Vec<Value>]) -> String {
    let entry = input
        .iter()
        .map(|particle| match &particle[..] {
            [] => "()".to_owned(),
            [Value::Rule { rule }] => definition(rule),
            _ => join(particle),
        })
        .collect::<Vec<_>>();
    format!("[{}]", entry.join(", "))
}

fn product(output: &[Output]) -> Option<String> {
    match output {
        [] => None,
        [single] => Some(item(single)),
        several => Some(format!(
            "({})",
            several.iter().map(item).collect::<Vec<_>>().join(", ")
        )),
    }
}

fn item(output: &Output) -> String {
    let Some(body) = &output.body else {
        return coherence(&output.particle);
    };
    let entry = (!output.particle.is_empty())
        .then(|| coherence(&output.particle))
        .into_iter()
        .chain(body.iter().map(definition))
        .collect::<Vec<_>>();
    format!("({})", entry.join(", "))
}

pub fn coherence(particle: &[Value]) -> String {
    match particle {
        [] => "()".to_owned(),
        [Value::Rule { .. }] => format!("().{}", join(particle)),
        _ => join(particle),
    }
}

fn join(particle: &[Value]) -> String {
    particle
        .iter()
        .map(|value| match value {
            Value::Atom(atom) => atom.clone(),
            Value::Rule { rule } => format!("({})", definition(rule)),
        })
        .collect::<Vec<_>>()
        .join(".")
}
