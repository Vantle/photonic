use crate::recording::Order;
use frontend::source::{Definition, Output, Program, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
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
        name: frontend::text::definition(&unnamed),
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

    pub fn show(&self, rule: &Definition) -> Definition {
        Definition {
            name: String::new(),
            ..definition(rule, &mut |atom| self.name(atom).to_owned())
        }
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

fn text(program: &Program) -> Canonical {
    let Program {
        mut initial,
        mut rule,
    } = program.canonical();
    rule.sort_by_cached_key(frontend::text::definition);
    initial.sort_by_cached_key(|entry| frontend::text::coherence(entry));
    Canonical {
        order: Order::Text,
        shape: None,
        program: named(&Program { initial, rule }),
        naming: Naming::default(),
    }
}

fn atom<'value>(particle: &'value [Value], name: &mut BTreeSet<&'value str>) {
    for value in particle {
        match value {
            Value::Atom(atom) => {
                name.insert(atom);
            }
            Value::Rule { rule } => self::rule(rule, name),
        }
    }
}

fn rule<'value>(definition: &'value Definition, name: &mut BTreeSet<&'value str>) {
    for particle in &definition.input {
        atom(particle, name);
    }
    for output in &definition.output {
        atom(&output.particle, name);
        for entry in output.body.iter().flatten() {
            rule(entry, name);
        }
    }
}

// The symmetry search breaks ties between interchangeable atoms by atom number, so numbering atoms
// by name instead of by first appearance keeps keys and handles independent of term order.
fn vocabulary(program: &Program) -> Option<Vocabulary> {
    let mut name = BTreeSet::new();
    for particle in &program.initial {
        atom(particle, &mut name);
    }
    for entry in &program.rule {
        rule(entry, &mut name);
    }
    Vocabulary::try_from(name.into_iter().map(str::to_owned).collect::<Vec<_>>()).ok()
}

fn shape(program: &Program) -> Option<Canonical> {
    let mut vocabulary = vocabulary(program)?;
    let (code, configuration) = lift::program(program, &mut vocabulary).ok()?;
    let structure = Structure {
        program: Part {
            program: code,
            configuration,
        },
        target: None,
        pin: Vec::new(),
    };
    let symmetry = structure.symmetry(BUDGET).ok()?;
    let letter = Vocabulary::alphabet(symmetry.atom.len()).ok()?;
    let part = &symmetry.form.program;
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
