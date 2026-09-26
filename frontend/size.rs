use crate::source::{Definition, Output, Program, Value};

pub(crate) fn definition(value: &Definition) -> usize {
    1 + value.name.len() + input(&value.input) + output(&value.output)
}

pub(crate) fn input(value: &[Vec<Value>]) -> usize {
    value.iter().map(|value| particle(value)).sum()
}

pub(crate) fn output(value: &[Output]) -> usize {
    value
        .iter()
        .map(|value| {
            1 + match value {
                Output::Particle(value) => particle(value),
                Output::Scope(value) => program(value),
            }
        })
        .sum()
}

fn program(value: &Program) -> usize {
    input(&value.initial)
        + value.rule.iter().map(definition).sum::<usize>()
        + value.scope.iter().map(program).sum::<usize>()
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
