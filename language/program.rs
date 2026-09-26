use frontend::source;
use hashing::Builder;
use indexmap::IndexSet;
use std::borrow::Cow;
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
pub enum Output {
    Particle(Vec<Symbol>),
    Scope(usize),
}

#[derive(Clone, Debug)]
pub struct Scope {
    pub name: String,
    pub initial: Vec<Vec<Symbol>>,
    pub rule: Vec<usize>,
    pub scope: Vec<usize>,
    pub opener: Option<usize>,
}

impl Scope {
    fn root(initial: Vec<Vec<Symbol>>, rule: Vec<usize>, scope: Vec<usize>) -> Self {
        Self {
            name: "root".into(),
            initial,
            rule,
            scope,
            opener: None,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Form {
    input: Vec<Vec<Symbol>>,
    output: Vec<Product>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum Product {
    Particle(Vec<Symbol>),
    Scope(Body),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Body {
    initial: Vec<Vec<Symbol>>,
    rule: Vec<usize>,
    scope: Vec<Self>,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub atom: IndexSet<String, Builder>,
    pub rule: Vec<Instruction>,
    pub scope: Vec<Scope>,
    interner: HashMap<Form, usize, Builder>,
    declaration: HashMap<(Option<usize>, Body), usize, Builder>,
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
            interner: HashMap::default(),
            declaration: HashMap::default(),
        };
        program.declare(source, "root".into(), None);
        program
    }

    // A target names the configuration its text loads as, so its scopes are ones a program opens at
    // the start: they share the declarations this program gives the same scopes, and hold nothing.
    pub(crate) fn target(&self, source: &source::Program) -> (Cow<'_, Self>, Scope) {
        if let Some(root) = self.known(source) {
            return (Cow::Borrowed(self), root);
        }
        let mut program = self.clone();
        let rule = source
            .rule
            .iter()
            .enumerate()
            .map(|(position, rule)| program.intern(rule, format!("root/{position}")))
            .collect();
        let initial = program.input(&source.initial);
        let scope = source
            .scope
            .iter()
            .enumerate()
            .map(|(position, value)| {
                program.declare(
                    value,
                    format!("root/{}", source.rule.len() + position),
                    None,
                )
            })
            .collect();
        (Cow::Owned(program), Scope::root(initial, rule, scope))
    }

    fn known(&self, source: &source::Program) -> Option<Scope> {
        let rule = source
            .rule
            .iter()
            .map(|rule| self.find(rule))
            .collect::<Option<Vec<_>>>()?;
        let initial = source
            .initial
            .iter()
            .map(|particle| {
                particle
                    .iter()
                    .map(|value| self.symbol(value))
                    .collect::<Option<Vec<_>>>()
            })
            .collect::<Option<Vec<_>>>()?;
        let scope = source
            .scope
            .iter()
            .map(|value| self.declaration.get(&(None, self.body(value)?)).copied())
            .collect::<Option<Vec<_>>>()?;
        Some(Scope::root(initial, rule, scope))
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
            .map(|output| match output {
                source::Output::Particle(particle) => {
                    self.multiset(particle).map(Product::Particle)
                }
                source::Output::Scope(program) => self.body(program).map(Product::Scope),
            })
            .collect::<Option<Vec<_>>>()?;
        output.sort_unstable();
        Some(output)
    }

    fn body(&self, value: &source::Program) -> Option<Body> {
        let mut rule = value
            .rule
            .iter()
            .map(|rule| self.find(rule))
            .collect::<Option<Vec<_>>>()?;
        rule.sort_unstable();
        let mut scope = value
            .scope
            .iter()
            .map(|scope| self.body(scope))
            .collect::<Option<Vec<_>>>()?;
        scope.sort_unstable();
        Some(Body {
            initial: self.pattern(&value.initial)?,
            rule,
            scope,
        })
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
        match output {
            Output::Particle(particle) => {
                let mut particle = particle.clone();
                particle.sort_unstable();
                Product::Particle(particle)
            }
            Output::Scope(scope) => Product::Scope(self.outline(*scope)),
        }
    }

    fn outline(&self, scope: usize) -> Body {
        let value = &self.scope[scope];
        let mut rule = value.rule.clone();
        rule.sort_unstable();
        let mut scope = value
            .scope
            .iter()
            .map(|&scope| self.outline(scope))
            .collect::<Vec<_>>();
        scope.sort_unstable();
        Body {
            initial: sorted(&value.initial),
            rule,
            scope,
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
            .map(|(position, output)| match output {
                source::Output::Particle(particle) => Output::Particle(self.particle(particle)),
                source::Output::Scope(program) => {
                    Output::Scope(self.declare(program, format!("{path}/{position}"), Some(index)))
                }
            })
            .collect();
        Instruction {
            name,
            input,
            output,
        }
    }

    fn declare(&mut self, value: &source::Program, name: String, opener: Option<usize>) -> usize {
        let scope = self.scope.len();
        self.scope.push(Scope {
            name: name.clone(),
            initial: Vec::new(),
            rule: Vec::new(),
            scope: Vec::new(),
            opener,
        });
        for (position, rule) in value.rule.iter().enumerate() {
            let index = self.intern(rule, format!("{name}/{position}"));
            self.scope[scope].rule.push(index);
        }
        self.scope[scope].initial = self.input(&value.initial);
        for (position, program) in value.scope.iter().enumerate() {
            let index = self.declare(
                program,
                format!("{name}/{}", value.rule.len() + position),
                opener,
            );
            self.scope[scope].scope.push(index);
        }
        // Identical scopes that one opener declares are one declaration, so their frames are
        // interchangeable as identical coherences are. A repeat found nothing new to intern, so
        // this entry, whose repeated nested scopes were already dropped, is all it added.
        let key = (opener, self.outline(scope));
        if let Some(&index) = self.declaration.get(&key) {
            self.scope.truncate(scope);
            return index;
        }
        self.declaration.insert(key, scope);
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
                .map(|particle| self.coherence(particle))
                .collect(),
            output: instruction
                .output
                .iter()
                .map(|output| match output {
                    Output::Particle(particle) => {
                        source::Output::Particle(self.coherence(particle))
                    }
                    Output::Scope(scope) => source::Output::Scope(self.declaration(*scope)),
                })
                .collect(),
        }
    }

    fn declaration(&self, scope: usize) -> source::Program {
        let value = &self.scope[scope];
        source::Program {
            initial: value
                .initial
                .iter()
                .map(|particle| self.coherence(particle))
                .collect(),
            rule: value
                .rule
                .iter()
                .map(|&rule| self.definition(rule))
                .collect(),
            scope: value
                .scope
                .iter()
                .map(|&scope| self.declaration(scope))
                .collect(),
        }
    }

    fn coherence(&self, particle: &[Symbol]) -> Vec<source::Value> {
        particle
            .iter()
            .map(|&symbol| self.express(symbol))
            .collect()
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
