use frontend::source;
use hashing::Builder;
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
    pub opener: Option<usize>,
}

pub(crate) struct Target {
    pub initial: Vec<Vec<Symbol>>,
    pub rule: Vec<usize>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Form {
    input: Vec<Vec<Symbol>>,
    output: Vec<Product>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Product {
    particle: Vec<Symbol>,
    body: Option<Vec<usize>>,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub atom: IndexSet<String, Builder>,
    pub rule: Vec<Instruction>,
    pub scope: Vec<Scope>,
    pub initial: Vec<Vec<Symbol>>,
    interner: HashMap<Form, usize, Builder>,
}

impl Program {
    pub(crate) fn contextual(&self) -> Vec<bool> {
        let mut contextual = vec![false; self.rule.len()];
        for &rule in self.scope.iter().flat_map(|scope| &scope.rule) {
            contextual[rule] = true;
        }
        contextual
    }

    pub fn new(source: &source::Program) -> Self {
        let mut program = Self {
            atom: IndexSet::default(),
            rule: Vec::new(),
            scope: Vec::new(),
            initial: Vec::new(),
            interner: HashMap::default(),
        };
        program.declare(&source.rule, "root".into(), None);
        program.initial = program.input(&source.initial);
        program
    }

    pub(crate) fn target(&self, source: &source::Program) -> Target {
        let rule = source
            .rule
            .iter()
            .map(|rule| self.find(rule))
            .collect::<Option<Vec<_>>>();
        let initial = source
            .initial
            .iter()
            .map(|particle| {
                particle
                    .iter()
                    .map(|value| self.symbol(value))
                    .collect::<Option<Vec<_>>>()
            })
            .collect::<Option<Vec<_>>>();
        if let (Some(rule), Some(initial)) = (rule, initial) {
            return Target { initial, rule };
        }
        let mut program = self.clone();
        let rule = source
            .rule
            .iter()
            .enumerate()
            .map(|(position, rule)| program.intern(rule, format!("root/{position}")))
            .collect();
        Target {
            initial: program.input(&source.initial),
            rule,
        }
    }

    fn symbol(&self, value: &source::Value) -> Option<Symbol> {
        match value {
            source::Value::Atom(atom) => self.atom.get_index_of(atom.as_str()).map(Symbol::Atom),
            source::Value::Rule { rule } => self.find(rule).map(Symbol::Rule),
        }
    }

    fn find(&self, value: &source::Definition) -> Option<usize> {
        self.interner.get(&self.form(value)?).copied()
    }

    fn form(&self, value: &source::Definition) -> Option<Form> {
        Some(Form {
            input: self.pattern(&value.input)?,
            output: self.sink(&value.output)?,
        })
    }

    fn pattern(&self, input: &[Vec<source::Value>]) -> Option<Vec<Vec<Symbol>>> {
        let mut input = input
            .iter()
            .map(|particle| self.multiset(particle))
            .collect::<Option<Vec<_>>>()?;
        input.sort_unstable();
        Some(input)
    }

    fn sink(&self, output: &[source::Output]) -> Option<Vec<Product>> {
        let mut output = output
            .iter()
            .map(|output| {
                let body = match &output.body {
                    Some(body) => {
                        let mut rule = body
                            .iter()
                            .map(|value| self.find(value))
                            .collect::<Option<Vec<_>>>()?;
                        rule.sort_unstable();
                        Some(rule)
                    }
                    None => None,
                };
                Some(Product {
                    particle: self.multiset(&output.particle)?,
                    body,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        output.sort_unstable();
        Some(output)
    }

    fn shape(&self, instruction: &Instruction) -> Form {
        let mut output = instruction
            .output
            .iter()
            .map(|output| self.product(output))
            .collect::<Vec<_>>();
        output.sort_unstable();
        Form {
            input: sorted(&instruction.input),
            output,
        }
    }

    fn product(&self, output: &Output) -> Product {
        let mut particle = output.particle.clone();
        particle.sort_unstable();
        Product {
            particle,
            body: output.body.map(|scope| {
                let mut rule = self.scope[scope].rule.clone();
                rule.sort_unstable();
                rule
            }),
        }
    }

    fn multiset(&self, particle: &[source::Value]) -> Option<Vec<Symbol>> {
        let mut symbol = particle
            .iter()
            .map(|value| self.symbol(value))
            .collect::<Option<Vec<_>>>()?;
        symbol.sort_unstable();
        Some(symbol)
    }

    fn value(&mut self, value: &source::Value) -> Symbol {
        match value {
            source::Value::Atom(atom) => Symbol::Atom(
                self.atom
                    .get_index_of(atom.as_str())
                    .unwrap_or_else(|| self.atom.insert_full(atom.clone()).0),
            ),
            source::Value::Rule { rule } => Symbol::Rule(self.find(rule).unwrap_or_else(|| {
                let index = self.rule.len();
                self.compile(
                    rule,
                    format!("Rule {}", index + 1),
                    format!("value/{index}"),
                )
            })),
        }
    }

    fn intern(&mut self, value: &source::Definition, path: String) -> usize {
        self.find(value)
            .unwrap_or_else(|| self.compile(value, path.clone(), path))
    }

    fn compile(&mut self, value: &source::Definition, name: String, path: String) -> usize {
        let index = self.rule.len();
        self.rule.push(Instruction::default());
        self.rule[index] = self.instruction(value, name, path, index);
        let form = self.shape(&self.rule[index]);
        self.interner.insert(form, index);
        index
    }

    fn particle(&mut self, value: &[source::Value]) -> Vec<Symbol> {
        value.iter().map(|value| self.value(value)).collect()
    }

    fn input(&mut self, value: &[Vec<source::Value>]) -> Vec<Vec<Symbol>> {
        value.iter().map(|value| self.particle(value)).collect()
    }

    fn instruction(
        &mut self,
        value: &source::Definition,
        name: String,
        path: String,
        index: usize,
    ) -> Instruction {
        let name = if value.name.is_empty() {
            name
        } else {
            value.name.clone()
        };
        let input = self.input(&value.input);
        let output = value
            .output
            .iter()
            .enumerate()
            .map(|(position, output)| Output {
                particle: self.particle(&output.particle),
                body: output
                    .body
                    .as_ref()
                    .map(|value| self.declare(value, format!("{path}/{position}"), Some(index))),
            })
            .collect();
        Instruction {
            name,
            input,
            output,
        }
    }

    fn declare(
        &mut self,
        value: &[source::Definition],
        name: String,
        opener: Option<usize>,
    ) -> usize {
        let scope = self.scope.len();
        self.scope.push(Scope {
            name: name.clone(),
            rule: Vec::new(),
            opener,
        });
        for (position, value) in value.iter().enumerate() {
            let index = self.intern(value, format!("{name}/{position}"));
            self.scope[scope].rule.push(index);
        }
        scope
    }

    pub(crate) fn express(&self, symbol: Symbol) -> source::Value {
        match symbol {
            Symbol::Atom(index) => source::Value::Atom(self.atom[index].clone()),
            Symbol::Rule(index) => source::Value::Rule {
                rule: Box::new(self.definition(index)),
            },
        }
    }

    pub(crate) fn definition(&self, index: usize) -> source::Definition {
        let instruction = &self.rule[index];
        source::Definition {
            name: String::new(),
            input: instruction
                .input
                .iter()
                .map(|particle| {
                    particle
                        .iter()
                        .map(|&symbol| self.express(symbol))
                        .collect()
                })
                .collect(),
            output: instruction
                .output
                .iter()
                .map(|output| source::Output {
                    particle: output
                        .particle
                        .iter()
                        .map(|&symbol| self.express(symbol))
                        .collect(),
                    body: output.body.map(|scope| {
                        self.scope[scope]
                            .rule
                            .iter()
                            .map(|&rule| self.definition(rule))
                            .collect()
                    }),
                })
                .collect(),
        }
    }
}

fn sorted(input: &[Vec<Symbol>]) -> Vec<Vec<Symbol>> {
    let mut input = input
        .iter()
        .map(|particle| {
            let mut particle = particle.clone();
            particle.sort_unstable();
            particle
        })
        .collect::<Vec<_>>();
    input.sort_unstable();
    input
}

#[cfg(test)]
#[path = "test/program.rs"]
mod test;
