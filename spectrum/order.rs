use crate::recording::Order;
use photonic::parser::{DELIMITER, SPACE};
use photonic::source::{Definition, Output, Program, Value};
use std::collections::{BTreeMap, HashMap};
use symmetry::structure::{Part, Structure};
use translation::lift;
use translation::vocabulary::{Vocabulary, letter};

const BUDGET: usize = 100_000;

#[derive(Clone, Debug, Default, Hash)]
pub struct Naming {
    name: BTreeMap<String, String>,
    atom: BTreeMap<String, String>,
}

pub struct Canonical {
    pub order: Order,
    pub shape: Option<u64>,
    pub program: Program,
    pub naming: Naming,
}

fn delimiter(character: char) -> bool {
    SPACE.contains(&character) || DELIMITER.contains(&character)
}

fn rename(text: &str, map: impl Fn(&str) -> Option<String>) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(character) = rest.chars().next() {
        if delimiter(character) {
            result.push(character);
            rest = &rest[character.len_utf8()..];
            continue;
        }
        let end = rest.find(delimiter).unwrap_or(rest.len());
        let atom = &rest[..end];
        result.push_str(&map(atom).unwrap_or_else(|| atom.to_owned()));
        rest = &rest[end..];
    }
    result
}

fn value(value: &Value, map: &mut impl FnMut(&str) -> String) -> Value {
    match value {
        Value::Atom(atom) => Value::Atom(map(atom)),
        Value::Rule { rule } => Value::Rule {
            rule: Box::new(definition(rule, map)),
        },
    }
}

fn particle(particle: &[Value], map: &mut impl FnMut(&str) -> String) -> Vec<Value> {
    particle.iter().map(|entry| value(entry, map)).collect()
}

fn definition(definition: &Definition, map: &mut impl FnMut(&str) -> String) -> Definition {
    let unnamed = Definition {
        name: String::new(),
        input: definition
            .input
            .iter()
            .map(|entry| particle(entry, map))
            .collect(),
        output: definition
            .output
            .iter()
            .map(|output| Output {
                particle: particle(&output.particle, map),
                body: output.body.as_ref().map(|body| {
                    body.iter()
                        .map(|entry| self::definition(entry, map))
                        .collect()
                }),
            })
            .collect(),
    };
    Definition {
        name: photonic::text::definition(&unnamed),
        ..unnamed
    }
}

fn program(program: &Program, mut map: impl FnMut(&str) -> String) -> Program {
    Program {
        initial: program
            .initial
            .iter()
            .map(|entry| particle(entry, &mut map))
            .collect(),
        rule: program
            .rule
            .iter()
            .map(|entry| definition(entry, &mut map))
            .collect(),
    }
}

fn named(source: &Program) -> Program {
    program(source, str::to_owned)
}

impl Naming {
    fn new(name: Vec<String>) -> Self {
        let pair = name
            .into_iter()
            .enumerate()
            .map(|(position, name)| (letter(position), name))
            .collect::<Vec<_>>();
        Self {
            atom: pair
                .iter()
                .map(|(letter, name)| (name.clone(), letter.clone()))
                .collect(),
            name: pair.into_iter().collect(),
        }
    }

    pub fn name<'value>(&'value self, atom: &'value str) -> &'value str {
        self.name.get(atom).map_or(atom, String::as_str)
    }

    pub fn show(&self, text: &str) -> String {
        if self.name.is_empty() {
            return text.to_owned();
        }
        rename(text, |atom| self.name.get(atom).cloned())
    }

    pub fn hide(&self, source: &Program) -> Program {
        if self.atom.is_empty() {
            return source.clone();
        }
        let mut fresh = HashMap::<String, String>::new();
        let count = self.atom.len();
        program(source, |name| {
            if let Some(atom) = self.atom.get(name) {
                return atom.clone();
            }
            let next = count + fresh.len();
            fresh
                .entry(name.to_owned())
                .or_insert_with(|| letter(next))
                .clone()
        })
    }
}

fn canonical(particle: &[Value]) -> Vec<Value> {
    let mut result = particle
        .iter()
        .map(|entry| match entry {
            Value::Atom(atom) => Value::Atom(atom.clone()),
            Value::Rule { rule } => Value::Rule {
                rule: Box::new(rule.canonical()),
            },
        })
        .collect::<Vec<_>>();
    result.sort();
    result
}

fn text(program: &Program) -> Canonical {
    let mut rule = program
        .rule
        .iter()
        .map(Definition::canonical)
        .collect::<Vec<_>>();
    rule.sort_by_cached_key(photonic::text::definition);
    let mut initial = program
        .initial
        .iter()
        .map(|entry| canonical(entry))
        .collect::<Vec<_>>();
    initial.sort_by_cached_key(|entry| photonic::text::coherence(entry));
    Canonical {
        order: Order::Text,
        shape: None,
        program: named(&Program { initial, rule }),
        naming: Naming::default(),
    }
}

fn shape(program: &Program) -> Option<Canonical> {
    let mut vocabulary = Vocabulary::default();
    let (code, configuration) = lift::program(program, &mut vocabulary).ok()?;
    let structure = Structure {
        part: vec![Part {
            role: 0,
            program: code,
            configuration,
        }],
        pin: Vec::new(),
    };
    let symmetry = structure.symmetry(BUDGET).ok()?;
    let form = symmetry.form(&structure);
    let letter = Vocabulary::alphabet(symmetry.atom.len()).ok()?;
    let part = form.part.first()?;
    Some(Canonical {
        order: Order::Shape,
        shape: Some(symmetry.fingerprint()),
        program: named(&translation::emit::program(
            &part.program,
            &part.configuration,
            &letter,
        )),
        naming: Naming::new(
            symmetry
                .atom
                .iter()
                .map(|&atom| vocabulary.name(atom).to_owned())
                .collect(),
        ),
    })
}

pub fn exhaustive(program: &Program) -> Canonical {
    shape(program).unwrap_or_else(|| text(program))
}

pub fn source(program: &Program) -> Canonical {
    Canonical {
        order: Order::Source,
        shape: None,
        program: named(program),
        naming: Naming::default(),
    }
}
