use crate::source::{Definition, Output, Value};

pub(crate) fn definition(value: &Definition) -> usize {
    1 + value.name.len()
        + input(&value.input)
        + value.rest.iter().map(|value| input(value)).sum::<usize>()
        + output(&value.output)
}

pub(crate) fn input(value: &[Vec<Value>]) -> usize {
    value.iter().map(|value| particle(value)).sum()
}

pub(crate) fn output(value: &[Output]) -> usize {
    value
        .iter()
        .map(|value| {
            1 + particle(&value.particle)
                + value.body.iter().flatten().map(definition).sum::<usize>()
        })
        .sum()
}

pub(crate) fn particle(value: &[Value]) -> usize {
    1 + value
        .iter()
        .map(|value| match value {
            Value::Atom(value) => 1 + value.len(),
            Value::Rule { rule } => definition(rule),
        })
        .sum::<usize>()
}
