use crate::rule;
use code::output::Output;
use frontend::source::{Definition, Program};
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

fn named(rule: Vec<&Definition>) -> Vec<String> {
    rule.into_iter().map(frontend::text::definition).collect()
}

fn translate(
    program: &Program,
    vocabulary: &mut Vocabulary,
) -> Option<(Vec<Statement>, Vec<Vec<String>>)> {
    let mut statement = Vec::new();
    let mut name = Vec::new();
    for entry in &program.rule {
        statement.push(Statement::Rule(lift::rule(entry, vocabulary).ok()?));
        name.push(named(rule::definition(entry)));
    }
    for entry in &program.initial {
        statement.push(Statement::Coherence(
            lift::particle(entry, vocabulary).ok()?,
        ));
        name.push(named(rule::particle(entry)));
    }
    for entry in &program.scope {
        for output in lift::scope(entry, vocabulary).ok()? {
            statement.push(match output {
                Output::Particle(particle) => Statement::Coherence(particle),
                Output::Scope(scope) => Statement::Scope(scope),
            });
            name.push(named(rule::program(entry)));
        }
    }
    Some((statement, name))
}

fn attribute(class: &[symmetry::analysis::Class], name: &[Vec<String>]) -> Vec<Vec<String>> {
    let mut member = vec![None; name.len()];
    for (index, entry) in class.iter().enumerate() {
        for &position in &entry.statement {
            member[position] = Some(index);
        }
    }
    let mut owner = BTreeMap::<&str, BTreeSet<Option<usize>>>::new();
    for (position, entry) in name.iter().enumerate() {
        for text in entry {
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
    let (statement, name) = translate(program, &mut vocabulary)?;
    let result = analyze(
        &statement::structure(&statement),
        &statement,
        crate::limit::SYMMETRY,
    )
    .ok()?;
    let class = result.class(&statement);
    let rule = attribute(&class, &name);
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
