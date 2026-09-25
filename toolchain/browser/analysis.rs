use code::particle::Particle;
use photonic::source::{Definition, Program, Value};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use symmetry::analysis::{Kind, analyze};
use symmetry::statement::{self, Statement};
use translation::lift;
use translation::vocabulary::Vocabulary;

const BUDGET: usize = 100_000;

#[derive(Serialize)]
pub struct Analysis {
    size: String,
    class: Vec<Class>,
}

#[derive(Serialize)]
struct Class {
    kind: &'static str,
    part: Vec<Vec<String>>,
    rule: Vec<String>,
}

fn definition<'source>(definition: &'source Definition, name: &mut Vec<&'source str>) {
    if !definition.name.is_empty() {
        name.push(&definition.name);
    }
    for particle in &definition.input {
        self::particle(particle, name);
    }
    for output in &definition.output {
        self::particle(&output.particle, name);
        for nested in output.body.iter().flatten() {
            self::definition(nested, name);
        }
    }
}

fn particle<'source>(particle: &'source [Value], name: &mut Vec<&'source str>) {
    for value in particle {
        if let Value::Rule { rule } = value {
            definition(rule, name);
        }
    }
}

fn kind(kind: Kind) -> &'static str {
    match kind {
        Kind::Global => "global",
        Kind::Local => "local",
        Kind::Block => "block",
    }
}

fn statement(program: &Program, vocabulary: &mut Vocabulary) -> Option<Vec<Statement>> {
    let mut statement = Vec::new();
    for entry in &program.rule {
        statement.push(Statement::Rule(lift::rule(entry, vocabulary).ok()?));
    }
    for entry in &program.initial {
        let value = entry
            .iter()
            .map(|value| lift::value(value, vocabulary))
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        statement.push(Statement::Coherence(Particle::from(value)));
    }
    Some(statement)
}

fn name(program: &Program) -> Vec<Vec<&str>> {
    let rule = program.rule.iter().map(|entry| {
        let mut text = Vec::new();
        definition(entry, &mut text);
        text
    });
    let coherence = program.initial.iter().map(|entry| {
        let mut text = Vec::new();
        particle(entry, &mut text);
        text
    });
    rule.chain(coherence).collect()
}

fn rule(class: &[symmetry::analysis::Class], name: &[Vec<&str>]) -> Vec<Vec<String>> {
    let mut member = vec![None; name.len()];
    for (index, class) in class.iter().enumerate() {
        for &position in &class.statement {
            member[position] = Some(index);
        }
    }
    let mut owner = BTreeMap::<&str, BTreeSet<Option<usize>>>::new();
    for (position, text) in name.iter().enumerate() {
        for &text in text {
            owner.entry(text).or_default().insert(member[position]);
        }
    }
    let mut rule = vec![Vec::new(); class.len()];
    for (text, holder) in owner {
        if let (1, Some(&Some(index))) = (holder.len(), holder.first()) {
            rule[index].push(text.to_owned());
        }
    }
    rule
}

pub fn analysis(program: &Program) -> Option<Analysis> {
    let mut vocabulary = Vocabulary::default();
    let statement = self::statement(program, &mut vocabulary)?;
    let result = analyze(&statement::structure(&statement), &statement, BUDGET).ok()?;
    let class = result.class(&statement);
    let rule = self::rule(&class, &name(program));
    Some(Analysis {
        size: result.symmetry.size.to_string(),
        class: class
            .iter()
            .zip(rule)
            .map(|(class, rule)| Class {
                kind: kind(class.kind),
                part: class
                    .part
                    .iter()
                    .map(|part| {
                        part.iter()
                            .map(|&atom| vocabulary.name(atom).to_owned())
                            .collect()
                    })
                    .collect(),
                rule,
            })
            .collect(),
    })
}
