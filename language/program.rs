use crate::source;
use indexmap::IndexSet;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Symbol {
    Atom(usize),
    Rule(usize),
}

#[derive(Clone, Debug, Default)]
pub struct Instruction {
    pub name: String,
    pub input: Vec<Vec<Symbol>>,
    pub output: Vec<Output>,
}

#[derive(Clone, Debug)]
pub struct Output {
    pub particle: Vec<Symbol>,
    pub body: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Scope {
    pub name: String,
    pub rule: Vec<usize>,
    anchor: HashMap<Symbol, Vec<usize>>,
    empty: Vec<usize>,
}

impl Scope {
    pub(crate) fn candidate(&self, available: impl IntoIterator<Item = Symbol>) -> Vec<usize> {
        let mut candidate = self.empty.clone();
        for symbol in available {
            if let Some(rule) = self.anchor.get(&symbol) {
                candidate.extend_from_slice(rule);
            }
        }
        candidate.sort_unstable();
        candidate
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    pub atom: IndexSet<String>,
    pub rule: Vec<Instruction>,
    pub scope: Vec<Scope>,
    pub initial: Vec<Vec<Symbol>>,
    pub code: HashMap<usize, usize>,
    interner: HashMap<source::Definition, usize>,
}

impl Program {
    pub fn new(source: source::Program) -> Self {
        let mut program = Self {
            atom: IndexSet::new(),
            rule: Vec::new(),
            scope: Vec::new(),
            initial: Vec::new(),
            code: HashMap::new(),
            interner: HashMap::new(),
        };
        program.declare(&source.rule, "root".into());
        program.initial = source
            .initial
            .iter()
            .map(|value| program.particle(value))
            .collect();
        for scope in &mut program.scope {
            let mut frequency = HashMap::<Symbol, usize>::new();
            for &rule in &scope.rule {
                for &symbol in program.rule[rule].input.iter().flatten() {
                    *frequency.entry(symbol).or_default() += 1;
                }
            }
            for &rule in &scope.rule {
                match program.rule[rule]
                    .input
                    .iter()
                    .flatten()
                    .min_by_key(|&&symbol| (frequency[&symbol], symbol))
                {
                    Some(&symbol) => scope.anchor.entry(symbol).or_default().push(rule),
                    None => scope.empty.push(rule),
                }
            }
        }
        program
    }

    fn value(&mut self, value: &source::Value) -> Symbol {
        match value {
            source::Value::Atom(atom) => Symbol::Atom(self.atom.insert_full(atom.clone()).0),
            source::Value::Rule { rule } => {
                let canonical = rule.canonical();
                if let Some(index) = self.interner.get(&canonical) {
                    return Symbol::Rule(*index);
                }
                let ordinal = self.interner.len();
                let index = self.rule.len();
                self.rule.push(Instruction::default());
                self.interner.insert(canonical.clone(), index);
                self.code.insert(index, ordinal);
                let mut value = canonical;
                value.name = if rule.name.is_empty() {
                    format!("Rule {}", ordinal + 1)
                } else {
                    rule.name.clone()
                };
                let compiled = self.instruction(&value, format!("value/{ordinal}"));
                self.rule[index] = compiled;
                Symbol::Rule(index)
            }
        }
    }

    fn particle(&mut self, value: &[source::Value]) -> Vec<Symbol> {
        value.iter().map(|value| self.value(value)).collect()
    }

    pub(crate) fn input(&mut self, value: &[Vec<source::Value>]) -> Vec<Vec<Symbol>> {
        value.iter().map(|value| self.particle(value)).collect()
    }

    fn instruction(&mut self, value: &source::Definition, path: String) -> Instruction {
        Instruction {
            name: if value.name.is_empty() {
                path.clone()
            } else {
                value.name.clone()
            },
            input: self.input(&value.input),
            output: value
                .output
                .iter()
                .enumerate()
                .map(|(position, output)| Output {
                    particle: self.particle(&output.particle),
                    body: output
                        .body
                        .as_ref()
                        .map(|value| self.declare(value, format!("{path}/{position}"))),
                })
                .collect(),
        }
    }

    fn declare(&mut self, value: &[source::Definition], name: String) -> usize {
        let scope = self.scope.len();
        self.scope.push(Scope {
            name: name.clone(),
            rule: Vec::new(),
            anchor: HashMap::new(),
            empty: Vec::new(),
        });
        for (position, value) in value.iter().enumerate() {
            let index = self.rule.len();
            self.rule.push(Instruction::default());
            let compiled = self.instruction(value, format!("{name}/{position}"));
            self.rule[index] = compiled;
            self.scope[scope].rule.push(index);
        }
        scope
    }

    pub fn label(&self, symbol: Symbol) -> String {
        match symbol {
            Symbol::Atom(index) => self.atom[index].clone(),
            Symbol::Rule(index) => format!("⟨{}⟩", self.rule[index].name),
        }
    }
}
