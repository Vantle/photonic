use frontend::source::{Definition, Program, Value};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use symmetry::analysis::analyze;
use symmetry::statement::{self, Statement};
use translation::lift;
use translation::vocabulary::Vocabulary;

#[derive(Serialize)]
pub struct Analysis {
    size: String,
    class: Vec<Class>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Global,
    Local,
    Block,
}

#[derive(Serialize)]
struct Class {
    kind: Kind,
    part: Vec<Vec<String>>,
    rule: Vec<String>,
}

impl From<symmetry::analysis::Kind> for Kind {
    fn from(kind: symmetry::analysis::Kind) -> Self {
        match kind {
            symmetry::analysis::Kind::Global => Self::Global,
            symmetry::analysis::Kind::Local => Self::Local,
            symmetry::analysis::Kind::Block => Self::Block,
        }
    }
}

fn visit<'source>(rule: &'source Definition, name: &mut Vec<&'source str>) {
    if !rule.name.is_empty() {
        name.push(&rule.name);
    }
    for particle in &rule.input {
        scan(particle, name);
    }
    for output in &rule.output {
        scan(&output.particle, name);
        for nested in output.body.iter().flatten() {
            visit(nested, name);
        }
    }
}

fn scan<'source>(particle: &'source [Value], name: &mut Vec<&'source str>) {
    for value in particle {
        if let Value::Rule { rule } = value {
            visit(rule, name);
        }
    }
}

fn translate(program: &Program, vocabulary: &mut Vocabulary) -> Option<Vec<Statement>> {
    let mut statement = Vec::new();
    for entry in &program.rule {
        statement.push(Statement::Rule(lift::rule(entry, vocabulary).ok()?));
    }
    for entry in &program.initial {
        statement.push(Statement::Coherence(
            lift::particle(entry, vocabulary).ok()?,
        ));
    }
    Some(statement)
}

fn mention(program: &Program) -> Vec<Vec<&str>> {
    let rule = program.rule.iter().map(|entry| {
        let mut name = Vec::new();
        visit(entry, &mut name);
        name
    });
    let coherence = program.initial.iter().map(|entry| {
        let mut name = Vec::new();
        scan(entry, &mut name);
        name
    });
    rule.chain(coherence).collect()
}

fn attribute(class: &[symmetry::analysis::Class], name: &[Vec<&str>]) -> Vec<Vec<String>> {
    let mut member = vec![None; name.len()];
    for (index, entry) in class.iter().enumerate() {
        for &position in &entry.statement {
            member[position] = Some(index);
        }
    }
    let mut owner = BTreeMap::<&str, BTreeSet<Option<usize>>>::new();
    for (position, entry) in name.iter().enumerate() {
        for &text in entry {
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
    let statement = translate(program, &mut vocabulary)?;
    let result = analyze(
        &statement::structure(&statement),
        &statement,
        crate::limit::SYMMETRY,
    )
    .ok()?;
    let class = result.class(&statement);
    let rule = attribute(&class, &mention(program));
    Some(Analysis {
        size: result.symmetry.size.to_string(),
        class: class
            .iter()
            .zip(rule)
            .map(|(entry, name)| Class {
                kind: Kind::from(entry.kind),
                part: entry
                    .part
                    .iter()
                    .map(|part| {
                        part.iter()
                            .map(|&atom| vocabulary.name(atom).to_owned())
                            .collect()
                    })
                    .collect(),
                rule: name,
            })
            .collect(),
    })
}
