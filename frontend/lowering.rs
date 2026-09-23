use std::cell::Cell;

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
    #[error("Photonic nesting exceeds {limit} levels")]
    #[diagnostic(code(photonic::depth))]
    Depth {
        limit: usize,
        #[label("nesting limit exceeded here")]
        span: SourceSpan,
    },
    #[error("Photonic expansion exceeds its {limit}-unit frontend budget")]
    #[diagnostic(code(photonic::expansion))]
    Expansion {
        limit: usize,
        #[label("shared-prefix expansion exceeds the frontend budget")]
        span: SourceSpan,
    },
    #[error("a body accepts at most one initial coherence; found {count}")]
    #[diagnostic(code(photonic::body))]
    Body {
        count: usize,
        #[label("use one particle as the body's explicit initial output")]
        span: SourceSpan,
    },
}

struct Reader<'tree, 'source> {
    tree: &'tree Tree<'source>,
    index: &'tree [Vec<usize>],
    child: &'tree [usize],
    position: usize,
    depth: usize,
    budget: &'tree Cell<usize>,
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
    let mut index = vec![Vec::new(); tree.node().len()];
    for (position, node) in tree.node().iter().enumerate() {
        if let Some(parent) = node.parent {
            index[parent].push(position);
        }
    }
    let budget = Cell::new(BUDGET);
    Reader::new(&tree, &index, 0, 0, &budget).program()
}

impl<'tree, 'source> Reader<'tree, 'source> {
    fn new(
        tree: &'tree Tree<'source>,
        index: &'tree [Vec<usize>],
        parent: usize,
        depth: usize,
        budget: &'tree Cell<usize>,
    ) -> Self {
        Self {
            tree,
            index,
            child: &index[parent],
            position: 0,
            depth,
            budget,
        }
    }

    fn peek(&self) -> Option<Kind> {
        self.child
            .get(self.position)
            .map(|&index| self.tree.node()[index].kind)
    }

    fn space(&mut self) {
        while self.peek() == Some(Kind::Space) {
            self.position += 1;
        }
    }

    fn failure(&self, message: &str) -> Failure {
        let span = self
            .child
            .get(self.position)
            .map(|&index| self.tree.node()[index].span.clone())
            .unwrap_or(self.tree.source().len()..self.tree.source().len());
        Failure::Syntax {
            message: message.into(),
            span: (span.start, span.len()).into(),
        }
    }

    fn program(&mut self) -> Result<Program, Failure> {
        let mut program = Program::default();
        let mut start = 0;
        loop {
            self.space();
            match self.peek() {
                None => return Ok(program),
                Some(Kind::Coherence) => {
                    self.position += 1;
                    start = program.initial.len();
                }
                Some(Kind::Context) => program.rule.push(self.definition()?),
                _ => {
                    let particle = self.particle()?;
                    let previous = program.initial.split_off(start);
                    program.initial.extend(if previous.is_empty() {
                        particle
                    } else {
                        self.combine(previous, particle)?
                    });
                }
            }
        }
    }

    fn configuration(&mut self) -> Result<Vec<Vec<Value>>, Failure> {
        let mut result = Vec::new();
        self.space();
        if self.peek().is_none() {
            return Ok(result);
        }
        loop {
            result.extend(
                if self.peek() == Some(Kind::Coherence) || self.peek().is_none() {
                    vec![Vec::new()]
                } else {
                    self.particle()?
                },
            );
            self.space();
            match self.peek() {
                None => return Ok(result),
                Some(Kind::Coherence) => {
                    self.position += 1;
                    self.space();
                }
                _ => {
                    return Err(self
                        .failure("combine values with a dot or separate coherences with a comma"));
                }
            }
        }
    }

    fn bound(&self, index: usize) -> Result<(), Failure> {
        if self.depth < crate::parser::DEPTH {
            return Ok(());
        }
        Err(Failure::Depth {
            limit: crate::parser::DEPTH,
            span: (self.tree.node()[index].span.start, 1).into(),
        })
    }

    fn nested(&self, index: usize) -> Result<Self, Failure> {
        self.bound(index)?;
        Ok(Self::new(
            self.tree,
            self.index,
            index,
            self.depth + 1,
            self.budget,
        ))
    }

    fn definition(&mut self) -> Result<Definition, Failure> {
        self.bound(self.child[self.position])?;
        self.depth += 1;
        let result = self.rule();
        self.depth -= 1;
        result
    }

    fn rule(&mut self) -> Result<Definition, Failure> {
        let index = self.child[self.position];
        self.position += 1;
        let input =
            Reader::new(self.tree, self.index, index, self.depth, self.budget).configuration()?;
        self.space();
        let mut output = Vec::new();
        if self.peek().is_some() && self.peek() != Some(Kind::Coherence) {
            output.extend(self.destination()?);
            loop {
                self.space();
                if self.peek() == Some(Kind::Group) {
                    output.extend(self.destination()?);
                    continue;
                }
                if self.peek() != Some(Kind::Coherence) {
                    break;
                }
                let saved = self.position;
                self.position += 1;
                self.space();
                if self.peek().is_none() || self.peek() == Some(Kind::Context) {
                    self.position = saved;
                    break;
                }
                output.extend(self.destination()?);
            }
        }
        let start = self.tree.node()[index].span.start;
        let end = self.tree.node()[self.child[self.position - 1]].span.end;
        Ok(Definition {
            name: self.tree.source()[start..end].trim().to_owned(),
            input,
            output,
        })
    }

    fn destination(&mut self) -> Result<Vec<Output>, Failure> {
        self.space();
        if self.peek() != Some(Kind::Group) {
            return Ok(self
                .particle()?
                .into_iter()
                .map(|particle| Output {
                    particle,
                    body: None,
                })
                .collect());
        }
        let index = self.child[self.position];
        self.position += 1;
        let program = self.nested(index)?.program()?;
        if !program.rule.is_empty() {
            if program.initial.len() > 1 {
                let span = self.tree.node()[index].span.clone();
                return Err(Failure::Body {
                    count: program.initial.len(),
                    span: (span.start, span.len()).into(),
                });
            }
            return Ok(vec![Output {
                particle: program.initial.into_iter().next().unwrap_or_default(),
                body: Some(program.rule),
            }]);
        }
        let particle = if program.initial.is_empty() {
            vec![Vec::new()]
        } else {
            program.initial
        };
        Ok(self
            .tail(particle, false)?
            .into_iter()
            .map(|particle| Output {
                particle,
                body: None,
            })
            .collect())
    }

    fn combine(
        &self,
        left: Vec<Vec<Value>>,
        right: Vec<Vec<Value>>,
    ) -> Result<Vec<Vec<Value>>, Failure> {
        crate::expansion::combine(left, right, self.budget).ok_or_else(|| {
            let span = self
                .child
                .get(self.position.saturating_sub(1))
                .map(|&index| self.tree.node()[index].span.clone())
                .unwrap_or(0..0);
            Failure::Expansion {
                limit: BUDGET,
                span: (span.start, span.len()).into(),
            }
        })
    }

    fn particle(&mut self) -> Result<Vec<Vec<Value>>, Failure> {
        let particle = self.value()?;
        self.tail(particle, true)
    }

    fn tail(
        &mut self,
        mut particle: Vec<Vec<Value>>,
        mut adjacent: bool,
    ) -> Result<Vec<Vec<Value>>, Failure> {
        loop {
            let saved = self.position;
            self.space();
            if self.peek() == Some(Kind::Continuation) {
                self.position += 1;
                self.space();
                adjacent = true;
                let value = self.value()?;
                particle = self.combine(particle, value)?;
            } else if adjacent && saved == self.position && self.peek() == Some(Kind::Group) {
                let value = self.value()?;
                particle = self.combine(particle, value)?;
            } else {
                break;
            }
        }
        Ok(particle)
    }

    fn value(&mut self) -> Result<Vec<Vec<Value>>, Failure> {
        self.space();
        match self.peek() {
            Some(Kind::Concept) => {
                let index = self.child[self.position];
                self.position += 1;
                Ok(vec![vec![Value::Atom(
                    self.tree.source()[self.tree.node()[index].span.clone()].into(),
                )]])
            }
            Some(Kind::Context) => Ok(vec![vec![Value::Rule {
                rule: Box::new(self.definition()?),
            }]]),
            Some(Kind::Group) => {
                let index = self.child[self.position];
                self.position += 1;
                let program = self.nested(index)?.program()?;
                let particle = if program.initial.is_empty() {
                    vec![Vec::new()]
                } else {
                    program.initial
                };
                if program.rule.is_empty() {
                    return Ok(particle);
                }
                let rule = program
                    .rule
                    .into_iter()
                    .map(|rule| Value::Rule {
                        rule: Box::new(rule),
                    })
                    .collect();
                self.combine(particle, vec![rule])
            }
            _ => Err(self.failure("expected a concept, group, or source context")),
        }
    }
}
