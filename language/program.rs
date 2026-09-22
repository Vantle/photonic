use crate::hashing::Builder;
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
}

#[derive(Clone, Debug)]
pub struct Program {
    pub atom: IndexSet<String, Builder>,
    pub rule: Vec<Instruction>,
    pub scope: Vec<Scope>,
    pub initial: Vec<Vec<Symbol>>,
    interner: HashMap<source::Definition, usize, Builder>,
}

impl Program {
    pub fn new(source: source::Program) -> Self {
        let mut program = Self {
            atom: IndexSet::default(),
            rule: Vec::new(),
            scope: Vec::new(),
            initial: Vec::new(),
            interner: HashMap::default(),
        };
        program.declare(&source.rule, "root".into());
        program.initial = source
            .initial
            .iter()
            .map(|value| program.particle(value))
            .collect();
        program
    }

    pub(crate) fn target(&self, source: &source::Program) -> Self {
        let mut program = self.clone();
        program.scope[0].rule = source
            .rule
            .iter()
            .enumerate()
            .map(|(position, rule)| program.intern(rule, format!("root/{position}")))
            .collect();
        program.initial = program.input(&source.initial);
        program
    }

    fn value(&mut self, value: &source::Value) -> Symbol {
        match value {
            source::Value::Atom(atom) => Symbol::Atom(self.atom.insert_full(atom.clone()).0),
            source::Value::Rule { rule } => {
                let ordinal = self.rule.len();
                let mut rule = rule.as_ref().clone();
                if rule.name.is_empty() {
                    rule.name = format!("Rule {}", ordinal + 1);
                }
                Symbol::Rule(self.intern(&rule, format!("value/{ordinal}")))
            }
        }
    }

    fn intern(&mut self, value: &source::Definition, path: String) -> usize {
        let canonical = value.canonical();
        if let Some(&index) = self.interner.get(&canonical) {
            return index;
        }
        let index = self.rule.len();
        self.rule.push(Instruction::default());
        self.interner.insert(canonical, index);
        self.rule[index] = self.instruction(value, path);
        index
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
        });
        for (position, value) in value.iter().enumerate() {
            let index = self.intern(value, format!("{name}/{position}"));
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
