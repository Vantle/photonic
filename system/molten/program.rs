use crate::source;
use indexmap::IndexSet;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Symbol {
    Atom(usize),
    Variable(String),
    Structure(usize, Vec<Symbol>),
    Rule(Arc<Instruction>, Option<usize>),
}

#[derive(Clone, Debug, Default)]
pub struct Instruction {
    pub name: String,
    pub input: Vec<Vec<Symbol>>,
    pub output: Vec<Output>,
    pub negative: Option<Vec<Vec<Symbol>>>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Output {
    pub particle: Vec<Symbol>,
    pub body: Option<Arc<Scope>>,
}

#[derive(Clone, Debug, Default)]
pub struct Scope {
    pub name: String,
    pub rule: Vec<Arc<Instruction>>,
}

impl PartialEq for Instruction {
    fn eq(&self, other: &Self) -> bool {
        self.input == other.input && self.output == other.output && self.negative == other.negative
    }
}
impl Eq for Instruction {}
impl PartialOrd for Instruction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Instruction {
    fn cmp(&self, other: &Self) -> Ordering {
        (&self.input, &self.output, &self.negative).cmp(&(
            &other.input,
            &other.output,
            &other.negative,
        ))
    }
}
impl Hash for Instruction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&self.input, &self.output, &self.negative).hash(state);
    }
}
impl PartialEq for Scope {
    fn eq(&self, other: &Self) -> bool {
        self.rule == other.rule
    }
}
impl Eq for Scope {}
impl PartialOrd for Scope {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Scope {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rule.cmp(&other.rule)
    }
}
impl Hash for Scope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.rule.hash(state);
    }
}

fn particle(value: &[Symbol], operation: &mut impl FnMut(&Symbol) -> Symbol) -> Vec<Symbol> {
    let mut value = value.iter().map(operation).collect::<Vec<_>>();
    value.sort();
    value
}

impl Symbol {
    pub fn rename(&self, mapping: &mut impl FnMut(usize) -> usize) -> Self {
        if self.capture().is_empty() {
            return self.clone();
        }
        match self {
            Self::Atom(_) | Self::Variable(_) => self.clone(),
            Self::Structure(tag, content) => {
                Self::Structure(*tag, particle(content, &mut |value| value.rename(mapping)))
            }
            Self::Rule(rule, capture) => {
                let rule = rule.rename(mapping);
                let rule = if capture.is_some() {
                    crate::hygiene::instruction(&rule)
                } else {
                    rule
                };
                Self::Rule(Arc::new(rule), capture.map(mapping))
            }
        }
    }

    pub fn capture(&self) -> Vec<usize> {
        match self {
            Self::Atom(_) | Self::Variable(_) => Vec::new(),
            Self::Structure(_, content) => content.iter().flat_map(Self::capture).collect(),
            Self::Rule(rule, capture) => capture.iter().copied().chain(rule.capture()).collect(),
        }
    }

    pub fn close(&self, frame: usize) -> Self {
        match self {
            Self::Atom(_) | Self::Variable(_) => self.clone(),
            Self::Structure(tag, content) => {
                Self::Structure(*tag, particle(content, &mut |value| value.close(frame)))
            }
            Self::Rule(_, Some(_)) => self.clone(),
            Self::Rule(rule, None) => {
                Self::Rule(Arc::new(crate::hygiene::instruction(rule)), Some(frame))
            }
        }
    }

    pub fn substitute(&self, binding: &BTreeMap<String, Symbol>) -> Self {
        match self {
            Self::Atom(_) => self.clone(),
            Self::Variable(name) => binding.get(name).cloned().unwrap_or_else(|| self.clone()),
            Self::Structure(tag, content) => Self::Structure(
                *tag,
                particle(content, &mut |value| value.substitute(binding)),
            ),
            Self::Rule(_, Some(_)) => self.clone(),
            Self::Rule(rule, None) => Self::Rule(Arc::new(rule.substitute(binding)), None),
        }
    }

    pub fn binding(&self) -> BTreeSet<String> {
        match self {
            Self::Atom(_) => BTreeSet::new(),
            Self::Variable(name) => BTreeSet::from([name.clone()]),
            Self::Structure(_, content) => content.iter().flat_map(Self::binding).collect(),
            Self::Rule(rule, _) => rule.mention(),
        }
    }

    fn available(&self, binding: &BTreeSet<String>) -> bool {
        match self {
            Self::Atom(_) => true,
            Self::Variable(name) => binding.contains(name),
            Self::Structure(_, content) => content.iter().all(|value| value.available(binding)),
            Self::Rule(rule, None) => rule.complete(binding),
            Self::Rule(rule, Some(_)) => rule.complete(&BTreeSet::new()),
        }
    }

    pub fn variable(&self) -> bool {
        match self {
            Self::Atom(_) => false,
            Self::Variable(_) => true,
            Self::Structure(_, content) => content.iter().any(Self::variable),
            Self::Rule(rule, _) => rule.variable(),
        }
    }
}

impl Instruction {
    fn transform(&self, operation: &mut impl FnMut(&Symbol) -> Symbol) -> Self {
        let input = |value: &[Vec<Symbol>], operation: &mut _| {
            let mut result = value
                .iter()
                .map(|value| particle(value, operation))
                .collect::<Vec<_>>();
            result.sort();
            result
        };
        let initial = input(&self.input, operation);
        let negative = self.negative.as_ref().map(|value| input(value, operation));
        let mut output = self
            .output
            .iter()
            .map(|output| Output {
                particle: particle(&output.particle, operation),
                body: output
                    .body
                    .as_ref()
                    .map(|scope| Arc::new(scope.transform(operation))),
            })
            .collect::<Vec<_>>();
        output.sort();
        Self {
            name: self.name.clone(),
            input: initial,
            output,
            negative,
        }
    }

    pub fn rename(&self, mapping: &mut impl FnMut(usize) -> usize) -> Self {
        if self.capture().is_empty() {
            return self.clone();
        }
        self.transform(&mut |value| value.rename(mapping))
    }
    pub fn capture(&self) -> Vec<usize> {
        self.input
            .iter()
            .flatten()
            .chain(self.negative.iter().flatten().flatten())
            .chain(self.output.iter().flat_map(|output| &output.particle))
            .flat_map(Symbol::capture)
            .chain(
                self.output
                    .iter()
                    .filter_map(|output| output.body.as_ref())
                    .flat_map(|scope| scope.capture()),
            )
            .collect()
    }
    pub fn close(&self, frame: usize) -> Self {
        self.transform(&mut |value| value.close(frame))
    }
    pub fn substitute(&self, binding: &BTreeMap<String, Symbol>) -> Self {
        self.transform(&mut |value| value.substitute(binding))
    }
    fn mention(&self) -> BTreeSet<String> {
        self.input
            .iter()
            .flatten()
            .chain(self.negative.iter().flatten().flatten())
            .chain(self.output.iter().flat_map(|output| &output.particle))
            .flat_map(Symbol::binding)
            .chain(
                self.output
                    .iter()
                    .filter_map(|output| output.body.as_ref())
                    .flat_map(|scope| scope.rule.iter().flat_map(|rule| rule.mention())),
            )
            .collect()
    }

    pub fn binding(&self) -> BTreeSet<String> {
        self.input
            .iter()
            .flatten()
            .flat_map(Symbol::binding)
            .collect()
    }

    fn complete(&self, outer: &BTreeSet<String>) -> bool {
        let binding = outer.union(&self.binding()).cloned().collect();
        self.available(&binding)
    }

    fn available(&self, binding: &BTreeSet<String>) -> bool {
        self.output.iter().all(|output| {
            output.particle.iter().all(|value| value.available(binding))
                && output
                    .body
                    .as_ref()
                    .is_none_or(|scope| scope.rule.iter().all(|rule| rule.complete(binding)))
        })
    }

    pub fn ready(&self) -> bool {
        self.available(&BTreeSet::new())
    }

    pub fn variable(&self) -> bool {
        self.input
            .iter()
            .flatten()
            .chain(self.negative.iter().flatten().flatten())
            .chain(self.output.iter().flat_map(|output| &output.particle))
            .any(Symbol::variable)
            || self
                .output
                .iter()
                .filter_map(|output| output.body.as_ref())
                .any(|scope| scope.rule.iter().any(|rule| rule.variable()))
    }
}

impl Scope {
    fn transform(&self, operation: &mut impl FnMut(&Symbol) -> Symbol) -> Self {
        let mut rule = self
            .rule
            .iter()
            .map(|rule| Arc::new(rule.transform(operation)))
            .collect::<Vec<_>>();
        rule.sort();
        Self {
            name: self.name.clone(),
            rule,
        }
    }
    pub fn rename(&self, mapping: &mut impl FnMut(usize) -> usize) -> Self {
        if self.capture().is_empty() {
            return crate::hygiene::scope(self);
        }
        crate::hygiene::scope(&self.transform(&mut |value| value.rename(mapping)))
    }
    pub fn capture(&self) -> Vec<usize> {
        self.rule.iter().flat_map(|rule| rule.capture()).collect()
    }
    pub fn close(&self, frame: usize) -> Self {
        self.transform(&mut |value| value.close(frame))
    }
    pub fn substitute(&self, binding: &BTreeMap<String, Symbol>) -> Self {
        crate::hygiene::scope(&self.transform(&mut |value| value.substitute(binding)))
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    pub atom: IndexSet<String>,
    pub rule: Vec<Arc<Instruction>>,
    pub scope: Vec<Arc<Scope>>,
    pub initial: Vec<Vec<Symbol>>,
    pub code: HashMap<Arc<Instruction>, usize>,
    reservation: HashMap<source::Definition, usize>,
}

impl Program {
    pub fn new(source: source::Program) -> Self {
        let mut program = Self {
            atom: IndexSet::new(),
            rule: Vec::new(),
            scope: Vec::new(),
            initial: Vec::new(),
            code: HashMap::new(),
            reservation: HashMap::new(),
        };
        program.declare(&source.rule, "root".into());
        program.initial = source
            .initial
            .iter()
            .map(|value| program.particle(value))
            .collect();
        program.scope[0] = Arc::new(crate::hygiene::scope(&program.scope[0]));
        let mut code = std::mem::take(&mut program.code)
            .into_iter()
            .collect::<Vec<_>>();
        code.sort_by_key(|(_, ordinal)| *ordinal);
        for (rule, ordinal) in code {
            program
                .code
                .entry(Arc::new(crate::hygiene::instruction(&rule)))
                .or_insert(ordinal);
        }
        program
    }

    fn value(&mut self, value: &source::Value) -> Symbol {
        match value {
            source::Value::Atom(atom) => Symbol::Atom(self.atom.insert_full(atom.clone()).0),
            source::Value::Variable { variable } => Symbol::Variable(variable.clone()),
            source::Value::Structure {
                structure,
                particle,
            } => Symbol::Structure(
                self.atom.insert_full(structure.clone()).0,
                self.particle(particle),
            ),
            source::Value::Rule { rule } => {
                let mut canonical = rule.canonical();
                let ordinal = self.reservation.len();
                let ordinal = *self.reservation.entry(canonical.clone()).or_insert(ordinal);
                canonical.name = if rule.name.is_empty() {
                    format!("Rule {}", ordinal + 1)
                } else {
                    rule.name.clone()
                };
                let compiled = Arc::new(self.instruction(&canonical, format!("value/{ordinal}")));
                self.code.entry(compiled.clone()).or_insert(ordinal);
                Symbol::Rule(compiled, None)
            }
        }
    }

    fn particle(&mut self, value: &[source::Value]) -> Vec<Symbol> {
        let mut result = value
            .iter()
            .map(|value| self.value(value))
            .collect::<Vec<_>>();
        result.sort();
        result
    }

    fn input(&mut self, value: &[Vec<source::Value>]) -> Vec<Vec<Symbol>> {
        let mut result = value
            .iter()
            .map(|value| self.particle(value))
            .collect::<Vec<_>>();
        result.sort();
        result
    }

    fn instruction(&mut self, value: &source::Definition, path: String) -> Instruction {
        let input = self.input(&value.input);
        let negative = value.negative.as_ref().map(|value| self.input(value));
        let mut output = value
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
            .collect::<Vec<_>>();
        output.sort();
        Instruction {
            name: if value.name.is_empty() {
                path
            } else {
                value.name.clone()
            },
            input,
            output,
            negative,
        }
    }

    fn declare(&mut self, value: &[source::Definition], name: String) -> Arc<Scope> {
        let scope = self.scope.len();
        self.scope.push(Arc::new(Scope::default()));
        let mut rule = Vec::new();
        for (position, value) in value.iter().enumerate() {
            let compiled = Arc::new(self.instruction(value, format!("{name}/{position}")));
            self.rule.push(compiled.clone());
            rule.push(compiled);
        }
        rule.sort();
        let result = Arc::new(Scope { name, rule });
        self.scope[scope] = result.clone();
        result
    }

    pub fn render(&self, rule: &Instruction) -> String {
        let particle = |value: &[Symbol]| {
            if value.is_empty() {
                return "()".to_owned();
            }
            value
                .iter()
                .map(|value| self.label(value))
                .collect::<Vec<_>>()
                .join(".")
        };
        let input = |value: &[Vec<Symbol>]| {
            format!(
                "[{}]",
                value
                    .iter()
                    .map(|value| particle(value))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let negative = rule
            .negative
            .as_ref()
            .map(|value| format!(" unless {}", input(value)))
            .unwrap_or_default();
        let output = if rule.output.is_empty() {
            "[]".to_owned()
        } else {
            rule.output
                .iter()
                .map(|output| {
                    let Some(body) = &output.body else {
                        return particle(&output.particle);
                    };
                    let mut content = Vec::new();
                    if !output.particle.is_empty() {
                        content.push(particle(&output.particle));
                    }
                    content.extend(body.rule.iter().map(|rule| self.render(rule)));
                    if content.is_empty() {
                        "{}".to_owned()
                    } else {
                        format!("{{ {}; }}", content.join("; "))
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!("{}{negative} -> {output}", input(&rule.input))
    }

    pub fn label(&self, symbol: &Symbol) -> String {
        match symbol {
            Symbol::Atom(index) => self.atom[*index].clone(),
            Symbol::Variable(name) => format!("${}", name.strip_prefix('$').unwrap_or(name)),
            Symbol::Structure(tag, content) => format!(
                "{}({})",
                self.atom[*tag],
                content
                    .iter()
                    .map(|value| self.label(value))
                    .collect::<Vec<_>>()
                    .join(".")
            ),
            Symbol::Rule(rule, _) => format!("@({})", self.render(rule)),
        }
    }
}
