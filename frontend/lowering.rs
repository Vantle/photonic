use std::ops::Range;

use miette::NamedSource;

use crate::failure::Failure;
use crate::partition::Partition;
use crate::source::{Definition, Output, Program, Value};
use crate::syntax::{Kind, Tree};

const BUDGET: usize = 1_000_000;

enum Member {
    Particle(Vec<Value>),
    Rule(Vec<Definition>),
    Scope(Output, Range<usize>),
}

struct Reader<'tree, 'source> {
    tree: &'tree Tree<'source>,
    child: &'tree [Vec<usize>],
    budget: usize,
}

pub fn read(path: &std::path::Path) -> miette::Result<Program> {
    let source = std::fs::read_to_string(path).map_err(|error| Failure::Read {
        path: path.display().to_string(),
        error,
    })?;
    parse(&source).map_err(|failure| {
        miette::Report::new(failure)
            .with_source_code(NamedSource::new(path.display().to_string(), source))
    })
}

pub fn parse(source: &str) -> Result<Program, Failure> {
    let tree = crate::parser::parse(source)?;
    let mut child = vec![Vec::new(); tree.node().len()];
    for (position, node) in tree.node().iter().enumerate() {
        if let Some(parent) = node.parent {
            child[parent].push(position);
        }
    }
    Reader {
        tree: &tree,
        child: &child,
        budget: BUDGET,
    }
    .program()
}

impl<'tree> Reader<'tree, '_> {
    fn child(&self, index: usize) -> &'tree [usize] {
        &self.child[index]
    }

    fn span(&self, index: usize) -> Range<usize> {
        self.tree.node()[index].span.clone()
    }

    fn kind(&self, index: usize) -> Kind {
        self.tree.node()[index].kind
    }

    fn failure(span: Range<usize>, message: &str) -> Failure {
        Failure::Lowering {
            message: message.into(),
            span: (span.start, span.len()).into(),
        }
    }

    fn expansion(span: Range<usize>) -> Failure {
        Failure::Expansion {
            limit: BUDGET,
            span: (span.start, span.len()).into(),
        }
    }

    fn program(&mut self) -> Result<Program, Failure> {
        let mut program = Program::default();
        for member in self.list(self.child(0)[0])? {
            match member {
                Member::Particle(particle) => program.initial.push(particle),
                Member::Rule(rule) => program.rule.extend(rule),
                Member::Scope(_, span) => {
                    return Err(Self::failure(
                        span,
                        "only a rule's output opens a scope; to keep a rule in a coherence, join it, as in ().([A] B)",
                    ));
                }
            }
        }
        Ok(program)
    }

    fn list(&mut self, index: usize) -> Result<Vec<Member>, Failure> {
        let mut result = Vec::new();
        for &term in self.child(index) {
            result.extend(self.term(term)?);
        }
        Ok(result)
    }

    fn bracketed(&self, index: usize) -> bool {
        self.child(index)
            .iter()
            .any(|&node| self.kind(node) == Kind::Rule)
    }

    fn term(&mut self, index: usize) -> Result<Vec<Member>, Failure> {
        if self.bracketed(index) {
            return Ok(vec![Member::Rule(self.partition(index)?)]);
        }
        let factor = self.child(index);
        self.body(factor)
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
        let member = self.list(self.child(index)[0])?;
        if member.is_empty() {
            return Ok(vec![Member::Particle(Vec::new())]);
        }
        if !member
            .iter()
            .any(|member| matches!(member, Member::Rule(_)))
        {
            return Ok(member);
        }
        Ok(vec![self.scope(index, member)?])
    }

    fn scope(&self, index: usize, member: Vec<Member>) -> Result<Member, Failure> {
        let span = self.span(index);
        let mut particle = Vec::new();
        let mut body = Vec::new();
        for member in member {
            match member {
                Member::Particle(value) => particle.push(value),
                Member::Rule(rule) => body.extend(rule),
                Member::Scope(_, span) => {
                    return Err(Self::failure(
                        span,
                        "a scope cannot hold another scope; to keep a rule in its coherence, join it, as in ().([A] B)",
                    ));
                }
            }
        }
        if particle.len() > 1 {
            return Err(Failure::Scope {
                count: particle.len(),
                span: (span.start, span.len()).into(),
            });
        }
        Ok(Member::Scope(
            Output {
                particle: particle.pop().unwrap_or_default(),
                body: Some(body),
            },
            span,
        ))
    }

    fn join(&mut self, factor: &[usize]) -> Result<Vec<Vec<Value>>, Failure> {
        let mut result = vec![Vec::new()];
        for &factor in factor {
            let value = self.factor(factor)?;
            result = crate::expansion::combine(result, value, &mut self.budget)
                .ok_or_else(|| Self::expansion(self.span(factor)))?;
        }
        Ok(result)
    }

    fn factor(&mut self, index: usize) -> Result<Vec<Vec<Value>>, Failure> {
        if self.kind(index) == Kind::Concept {
            return Ok(vec![vec![Value::Atom(
                self.tree.source()[self.span(index)].into(),
            )]]);
        }
        let term = self.child(self.child(index)[0]);
        if term.is_empty() {
            return Ok(vec![Vec::new()]);
        }
        let mut result = Vec::new();
        for &term in term {
            if self.bracketed(term) {
                result.push(self.partition(term)?.into_iter().map(value).collect());
                continue;
            }
            let factor = self.child(term);
            result.extend(self.join(factor)?);
        }
        Ok(result)
    }

    fn partition(&mut self, index: usize) -> Result<Vec<Definition>, Failure> {
        let child = self.child(index);
        let (rule, sink): (Vec<usize>, Vec<usize>) = child
            .iter()
            .partition(|&&node| self.kind(node) == Kind::Rule);
        let mut pattern = Vec::new();
        for node in rule {
            pattern.push(self.input(self.child(node)[0])?);
        }
        let mut output = Vec::new();
        for member in self.body(&sink)? {
            output.push(match member {
                Member::Particle(particle) => Output {
                    particle,
                    body: None,
                },
                Member::Scope(output, _) => output,
                Member::Rule(_) => unreachable!("a group that lists a rule is a scope"),
            });
        }
        let span = self.span(child[0]).start..self.span(child[child.len() - 1]).end;
        Partition {
            source: self.tree.source(),
            span: span.clone(),
            pattern,
            output,
        }
        .rule(&mut self.budget)
        .ok_or_else(|| Self::expansion(span))
    }

    fn input(&mut self, index: usize) -> Result<Vec<Vec<Value>>, Failure> {
        let mut input = Vec::new();
        for member in self.list(index)? {
            input.push(match member {
                Member::Particle(particle) => particle,
                Member::Rule(rule) => rule.into_iter().map(value).collect(),
                Member::Scope(_, span) => {
                    return Err(Self::failure(
                        span,
                        "an input cannot be a scope; match a rule without parentheses, as in [[A] B]",
                    ));
                }
            });
        }
        Ok(input)
    }
}

fn value(rule: Definition) -> Value {
    Value::Rule {
        rule: Box::new(rule),
    }
}
