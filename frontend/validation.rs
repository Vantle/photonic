use crate::source::{Definition, Output, Program, Value};

pub(crate) fn program(value: &Program) -> Result<(), &'static str> {
    for coherence in &value.initial {
        particle(coherence)?;
    }
    for rule in &value.rule {
        definition(rule)?;
    }
    for program in &value.scope {
        scope(program)?;
    }
    Ok(())
}

// Text writes a scope as a group that lists a rule, and a group that lists neither a coherence nor
// a scope holds the empty coherence, so a program read as data keeps to what text can write and
// means the same thing to the runtime, the printer and the code model.
fn scope(value: &Program) -> Result<(), &'static str> {
    if value.rule.is_empty() {
        return Err("a scope lists a rule; a group without one is only its members");
    }
    if value.initial.is_empty() && value.scope.is_empty() {
        return Err("a scope holds a coherence or a scope; write the empty coherence as []");
    }
    program(value)
}

fn definition(value: &Definition) -> Result<(), &'static str> {
    for coherence in &value.input {
        particle(coherence)?;
    }
    for output in &value.output {
        match output {
            Output::Particle(coherence) => particle(coherence)?,
            Output::Scope(program) => scope(program)?,
        }
    }
    Ok(())
}

fn particle(value: &[Value]) -> Result<(), &'static str> {
    for value in value {
        if let Value::Rule { rule } = value {
            definition(rule)?;
        }
    }
    Ok(())
}
