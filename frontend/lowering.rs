use std::cell::Cell;
use std::ops::Range;

use miette::{Diagnostic, IntoDiagnostic, NamedSource, SourceSpan, WrapErr};
use thiserror::Error;

use crate::source::{Definition, Output, Program, Value};
use crate::syntax::{Kind, Tree};

const BUDGET: usize = 1_000_000;

#[derive(Debug, Diagnostic, Error)]
pub enum Failure {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Parse(#[from] crate::failure::Failure),
    #[error("invalid Photonic expression: {message}")]
    #[diagnostic(code(photonic::lowering))]
    Syntax {
        message: String,
        #[label("{message}")]
        span: SourceSpan,
    },
    #[error("Photonic expansion exceeds its {limit}-unit frontend budget")]
    #[diagnostic(code(photonic::expansion))]
    Expansion {
        limit: usize,
        #[label("shared-prefix expansion exceeds the frontend budget")]
        span: SourceSpan,
    },
    #[error("a scope holds at most one coherence; found {count}")]
    #[diagnostic(code(photonic::scope))]
    Scope {
        count: usize,
        #[label("list one coherence beside the scope's rules")]
        span: SourceSpan,
    },
}

enum Member {
    Particle(Vec<Value>),
    Rule(Definition),
    Scope(Output, Range<usize>),
}

struct Reader<'tree, 'source> {
    tree: &'tree Tree<'source>,
    child: Vec<Vec<usize>>,
    budget: Cell<usize>,
}

pub fn read(path: &std::path::Path) -> miette::Result<Program> {
    let source = std::fs::read_to_string(path)
        .into_diagnostic()
        .wrap_err_with(|| format!("could not read {}", path.display()))?;
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
        child,
        budget: Cell::new(BUDGET),
    }
    .program()
}

impl Reader<'_, '_> {
    fn span(&self, index: usize) -> Range<usize> {
        self.tree.node()[index].span.clone()
    }

    fn failure(span: Range<usize>, message: &str) -> Failure {
        Failure::Syntax {
            message: message.into(),
            span: (span.start, span.len()).into(),
        }
    }

    fn program(&self) -> Result<Program, Failure> {
        let mut program = Program::default();
        for member in self.list(self.child[0][0])? {
            match member {
                Member::Particle(particle) => program.initial.push(particle),
                Member::Rule(rule) => program.rule.push(rule),
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

    fn list(&self, index: usize) -> Result<Vec<Member>, Failure> {
        let mut result = Vec::new();
        for &term in &self.child[index] {
            result.extend(self.term(term)?);
        }
        Ok(result)
    }

    fn term(&self, index: usize) -> Result<Vec<Member>, Failure> {
        if let [factor] = self.child[index][..] {
            return self.alone(factor);
        }
        Ok(self
            .join(index)?
            .into_iter()
            .map(Member::Particle)
            .collect())
    }

    fn alone(&self, index: usize) -> Result<Vec<Member>, Failure> {
        match self.tree.node()[index].kind {
            Kind::Rule => Ok(vec![Member::Rule(self.rule(index)?)]),
            Kind::Group => {
                let member = self.list(self.child[index][0])?;
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
            _ => Ok(self
                .factor(index)?
                .into_iter()
                .map(Member::Particle)
                .collect()),
        }
    }

    fn scope(&self, index: usize, member: Vec<Member>) -> Result<Member, Failure> {
        let span = self.span(index);
        let mut particle = Vec::new();
        let mut body = Vec::new();
        for member in member {
            match member {
                Member::Particle(value) => particle.push(value),
                Member::Rule(rule) => body.push(rule),
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

    fn join(&self, index: usize) -> Result<Vec<Vec<Value>>, Failure> {
        let mut result = vec![Vec::new()];
        for &factor in &self.child[index] {
            let value = self.factor(factor)?;
            result = crate::expansion::combine(result, value, &self.budget).ok_or_else(|| {
                let span = self.span(factor);
                Failure::Expansion {
                    limit: BUDGET,
                    span: (span.start, span.len()).into(),
                }
            })?;
        }
        Ok(result)
    }

    fn factor(&self, index: usize) -> Result<Vec<Vec<Value>>, Failure> {
        match self.tree.node()[index].kind {
            Kind::Concept => Ok(vec![vec![Value::Atom(
                self.tree.source()[self.span(index)].into(),
            )]]),
            Kind::Rule => Ok(vec![vec![Value::Rule {
                rule: Box::new(self.rule(index)?),
            }]]),
            _ => {
                let term = &self.child[self.child[index][0]];
                if term.is_empty() {
                    return Ok(vec![Vec::new()]);
                }
                let mut result = Vec::new();
                for &term in term {
                    result.extend(self.join(term)?);
                }
                Ok(result)
            }
        }
    }

    fn rule(&self, index: usize) -> Result<Definition, Failure> {
        let child = &self.child[index];
        let mut input = Vec::new();
        for member in self.list(child[0])? {
            input.push(match member {
                Member::Particle(particle) => particle,
                Member::Rule(rule) => vec![Value::Rule {
                    rule: Box::new(rule),
                }],
                Member::Scope(_, span) => {
                    return Err(Self::failure(
                        span,
                        "an input cannot be a scope; match a rule without parentheses, as in [[A] B]",
                    ));
                }
            });
        }
        let mut output = Vec::new();
        for &term in &child[1..] {
            for member in self.term(term)? {
                output.push(match member {
                    Member::Particle(particle) => Output {
                        particle,
                        body: None,
                    },
                    Member::Rule(rule) => Output {
                        particle: vec![Value::Rule {
                            rule: Box::new(rule),
                        }],
                        body: None,
                    },
                    Member::Scope(output, _) => output,
                });
            }
        }
        Ok(Definition {
            name: self.tree.source()[self.span(index)].trim().to_owned(),
            input,
            output,
        })
    }
}
