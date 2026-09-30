use std::ops::Range;

use miette::NamedSource;

use crate::failure::Failure;
use crate::partition::Partition;
use crate::source::{Definition, Library, Output, Program, Value};
use crate::syntax::{Kind, Tree};

const BUDGET: usize = 1_000_000;
// A large program distributes many small joins and partitions many small terms, each into a few
// times its text, so the budget grows with the source and only a term that multiplies itself runs
// out of it.
const RATIO: usize = 8;

enum Member {
    Particle(Vec<Value>),
    Rule(Vec<Definition>),
    Scope(Program, Range<usize>),
}

struct Reader<'tree, 'source> {
    tree: &'tree Tree<'source>,
    budget: usize,
}

pub fn read<Lowered>(
    path: &std::path::Path,
    lower: fn(&str) -> Result<Lowered, Failure>,
) -> miette::Result<Lowered> {
    let name = path.display().to_string();
    let byte = std::fs::read(path).map_err(|error| Failure::Read {
        path: name.clone(),
        error,
    })?;
    let source = crate::encoding::decode(&name, byte)?;
    lower(&source).map_err(|failure| {
        miette::Report::new(failure).with_source_code(NamedSource::new(name, source))
    })
}

pub fn parse(source: &str) -> Result<Program, Failure> {
    let tree = crate::parser::parse(source)?;
    Reader {
        tree: &tree,
        budget: allowance(source),
    }
    .program()
}

pub fn library(source: &str) -> Result<Library, Failure> {
    let tree = crate::parser::parse(source)?;
    let mut reader = Reader {
        tree: &tree,
        budget: allowance(source),
    };
    let mut rule = Vec::new();
    for term in tree.child(reader.first(0)) {
        for member in reader.term(term)? {
            let Member::Rule(definition) = member else {
                let span = reader.span(term);
                return Err(Failure::Library {
                    span: (span.start, span.len()).into(),
                });
            };
            rule.extend(definition);
        }
    }
    Ok(Library { rule })
}

fn allowance(source: &str) -> usize {
    BUDGET.saturating_add(source.len().saturating_mul(RATIO))
}

impl<'tree> Reader<'tree, '_> {
    fn child(&self, index: usize) -> Vec<usize> {
        self.tree.child(index).collect()
    }

    fn first(&self, index: usize) -> usize {
        self.tree
            .child(index)
            .next()
            .expect("every module, group and bracket holds a list")
    }

    fn span(&self, index: usize) -> Range<usize> {
        self.tree.node()[index].span.clone()
    }

    fn kind(&self, index: usize) -> Kind {
        self.tree.node()[index].kind
    }

    fn expansion(&self, span: Range<usize>) -> Failure {
        Failure::Expansion {
            limit: allowance(self.tree.source()),
            span: (span.start, span.len()).into(),
        }
    }

    fn program(&mut self) -> Result<Program, Failure> {
        let mut program = Program::default();
        for term in self.tree.child(self.first(0)) {
            for member in self.term(term)? {
                add(&mut program, member);
            }
        }
        Ok(program)
    }

    fn list(&mut self, index: usize) -> Result<Vec<Member>, Failure> {
        let mut result = Vec::new();
        for term in self.tree.child(index) {
            result.extend(self.term(term)?);
        }
        Ok(result)
    }

    fn bracketed(&self, index: usize) -> bool {
        self.tree
            .child(index)
            .any(|node| self.kind(node) == Kind::Rule)
    }

    fn term(&mut self, index: usize) -> Result<Vec<Member>, Failure> {
        if self.bracketed(index) {
            return Ok(vec![Member::Rule(self.partition(index)?)]);
        }
        let factor = self.child(index);
        self.body(&factor)
    }

    fn body(&mut self, factor: &[usize]) -> Result<Vec<Member>, Failure> {
        match factor {
            [] => Ok(Vec::new()),
            &[group] if self.kind(group) == Kind::Group => self.group(group),
            _ => Ok(self
                .join(factor)?
                .into_iter()
                .map(Member::Particle)
                .collect()),
        }
    }

    fn group(&mut self, index: usize) -> Result<Vec<Member>, Failure> {
        let member = self.list(self.first(index))?;
        if member.is_empty() {
            return Ok(vec![Member::Particle(Vec::new())]);
        }
        if !member
            .iter()
            .any(|member| matches!(member, Member::Rule(_)))
        {
            return Ok(member);
        }
        Ok(vec![self.scope(index, member)])
    }

    fn scope(&self, index: usize, member: Vec<Member>) -> Member {
        let mut program = assemble(member);
        // A scope that lists neither a coherence nor a scope holds the empty coherence, as () is the
        // empty coherence, so the remainder of the rule that opens a scope always has a coherence to
        // enter.
        if program.initial.is_empty() && program.scope.is_empty() {
            program.initial.push(Vec::new());
        }
        Member::Scope(program, self.span(index))
    }

    fn join(&mut self, factor: &[usize]) -> Result<Vec<Vec<Value>>, Failure> {
        let mut choice = Vec::<Vec<Vec<Value>>>::new();
        for &index in factor {
            let alternative = self.factor(index)?;
            match (choice.last_mut(), alternative.len()) {
                (Some(last), 1) if last.len() == 1 => {
                    last[0].extend(alternative.into_iter().flatten());
                }
                _ => choice.push(alternative),
            }
        }
        crate::expansion::product(choice, &mut self.budget).ok_or_else(|| {
            let start = factor.first().map_or(0, |&first| self.span(first).start);
            let end = factor.last().map_or(start, |&last| self.span(last).end);
            self.expansion(start..end)
        })
    }

    fn factor(&mut self, index: usize) -> Result<Vec<Vec<Value>>, Failure> {
        if self.kind(index) == Kind::Concept {
            return Ok(vec![vec![Value::Atom(
                self.tree.source()[self.span(index)].into(),
            )]]);
        }
        let term = self.child(self.first(index));
        if term.is_empty() {
            return Ok(vec![Vec::new()]);
        }
        let mut result = Vec::new();
        for term in term {
            if self.bracketed(term) {
                result.push(self.partition(term)?.into_iter().map(value).collect());
                continue;
            }
            let factor = self.child(term);
            result.extend(self.join(&factor)?);
        }
        Ok(result)
    }

    fn partition(&mut self, index: usize) -> Result<Vec<Definition>, Failure> {
        let child = self.child(index);
        let (rule, sink): (Vec<usize>, Vec<usize>) = child
            .iter()
            .partition(|&&node| self.kind(node) == Kind::Rule);
        let pattern = rule
            .into_iter()
            .map(|node| self.input(self.first(node)))
            .collect::<Result<_, _>>()?;
        let output = self
            .body(&sink)?
            .into_iter()
            .map(|member| match member {
                Member::Particle(particle) => Output::Particle(particle),
                Member::Scope(program, _) => Output::Scope(program),
                Member::Rule(_) => unreachable!("a group that lists a rule is a scope"),
            })
            .collect();
        Partition { pattern, output }
            .rule(&mut self.budget)
            .ok_or_else(|| {
                self.expansion(self.span(child[0]).start..self.span(child[child.len() - 1]).end)
            })
    }

    fn input(&mut self, index: usize) -> Result<Vec<Vec<Value>>, Failure> {
        self.list(index)?
            .into_iter()
            .map(|member| match member {
                Member::Particle(particle) => Ok(particle),
                Member::Rule(rule) => Ok(rule.into_iter().map(value).collect()),
                Member::Scope(_, span) => Err(Failure::Input {
                    span: (span.start, span.len()).into(),
                }),
            })
            .collect()
    }
}

fn value(rule: Definition) -> Value {
    Value::Rule {
        rule: Box::new(rule),
    }
}

fn assemble(member: Vec<Member>) -> Program {
    let mut program = Program::default();
    for member in member {
        add(&mut program, member);
    }
    program
}

fn add(program: &mut Program, member: Member) {
    match member {
        Member::Particle(particle) => program.initial.push(particle),
        Member::Rule(rule) => program.rule.extend(rule),
        Member::Scope(scope, _) => program.scope.push(scope),
    }
}
