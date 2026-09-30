use frontend::source::{Definition, Output, Program, Value};

// The rules a part of a program writes, in the order it writes them: a rule and the rules nested in
// its inputs and outputs, the rule values a particle holds, and everything a program or scope
// holds.
pub fn definition(definition: &Definition) -> Vec<&Definition> {
    let mut found = Vec::new();
    visit(definition, &mut found);
    found
}

pub fn particle(particle: &[Value]) -> Vec<&Definition> {
    let mut found = Vec::new();
    scan(particle, &mut found);
    found
}

pub fn program(program: &Program) -> Vec<&Definition> {
    let mut found = Vec::new();
    enclose(program, &mut found);
    found
}

fn visit<'program>(rule: &'program Definition, found: &mut Vec<&'program Definition>) {
    found.push(rule);
    for particle in &rule.input {
        scan(particle, found);
    }
    for output in &rule.output {
        match output {
            Output::Particle(particle) => scan(particle, found),
            Output::Scope(program) => enclose(program, found),
        }
    }
}

fn enclose<'program>(program: &'program Program, found: &mut Vec<&'program Definition>) {
    for particle in &program.initial {
        scan(particle, found);
    }
    for rule in &program.rule {
        visit(rule, found);
    }
    for nested in &program.scope {
        enclose(nested, found);
    }
}

fn scan<'program>(particle: &'program [Value], found: &mut Vec<&'program Definition>) {
    for value in particle {
        if let Value::Rule { rule } = value {
            visit(rule, found);
        }
    }
}
