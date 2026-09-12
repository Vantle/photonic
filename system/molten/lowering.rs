use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use crate::source::{Definition, Output, Program, Value};
use crate::syntax::{Kind, Tree};

#[derive(Debug, Diagnostic, Error)]
pub enum Failure {
    #[error("invalid Molten expression: {message}")]
    #[diagnostic(code(molten::lowering))]
    Syntax {
        message: String,
        #[label("{message}")]
        span: SourceSpan,
    },
    #[error("Molten nesting exceeds {limit} levels")]
    #[diagnostic(code(molten::depth))]
    Depth {
        limit: usize,
        #[label("nesting limit exceeded here")]
        span: SourceSpan,
    },
    #[error("a body accepts at most one initial coherence; found {count}")]
    #[diagnostic(code(molten::body))]
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
}

pub fn parse(source: &str) -> Result<Program, Failure> {
    let tree = crate::parser::parse(source).map_err(|error| match error {
        crate::failure::Failure::Syntax { message, span } => Failure::Syntax { message, span },
        crate::failure::Failure::Depth { limit, span } => Failure::Depth { limit, span },
    })?;
    let mut index = vec![Vec::new(); tree.node().len()];
    for (position, node) in tree.node().iter().enumerate() {
        if let Some(parent) = node.parent {
            index[parent].push(position);
        }
    }
    Reader::new(&tree, &index, 0, 0).program()
}

impl<'tree, 'source> Reader<'tree, 'source> {
    fn new(
        tree: &'tree Tree<'source>,
        index: &'tree [Vec<usize>],
        parent: usize,
        depth: usize,
    ) -> Self {
        Self {
            tree,
            index,
            child: &index[parent],
            position: 0,
            depth,
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
        let mut separate = true;
        loop {
            self.space();
            match self.peek() {
                None => return Ok(program),
                Some(Kind::Coherence) => {
                    self.position += 1;
                    separate = true;
                }
                Some(Kind::Context) => program.rule.push(self.definition()?),
                _ => {
                    let particle = self.particle()?;
                    if separate || program.initial.is_empty() {
                        program.initial.push(particle);
                    } else {
                        program.initial.last_mut().unwrap().extend(particle);
                    }
                    separate = false;
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
            result.push(
                if self.peek() == Some(Kind::Coherence) || self.peek().is_none() {
                    Vec::new()
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
        if self.depth < 128 {
            return Ok(());
        }
        Err(Failure::Depth {
            limit: 128,
            span: (self.tree.node()[index].span.start, 1).into(),
        })
    }

    fn nested(&self, index: usize) -> Result<Self, Failure> {
        self.bound(index)?;
        Ok(Self::new(self.tree, self.index, index, self.depth + 1))
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
        let input = Reader::new(self.tree, self.index, index, self.depth).configuration()?;
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
            return Ok(vec![Output {
                particle: self.particle()?,
                body: None,
            }]);
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
        let mut output = program
            .initial
            .into_iter()
            .map(|particle| Output {
                particle,
                body: None,
            })
            .collect::<Vec<_>>();
        if output.is_empty() {
            output.push(Output::default());
        }
        self.space();
        if self.peek() == Some(Kind::Continuation) {
            self.position += 1;
            let particle = self.particle()?;
            for item in &mut output {
                item.particle.extend(particle.clone());
            }
        }
        Ok(output)
    }

    fn particle(&mut self) -> Result<Vec<Value>, Failure> {
        let mut particle = self.value()?;
        loop {
            let saved = self.position;
            self.space();
            if self.peek() == Some(Kind::Continuation) {
                self.position += 1;
                self.space();
                particle.extend(self.value()?);
            } else if saved == self.position && self.peek() == Some(Kind::Group) {
                particle.extend(self.value()?);
            } else {
                break;
            }
        }
        Ok(particle)
    }

    fn value(&mut self) -> Result<Vec<Value>, Failure> {
        self.space();
        match self.peek() {
            Some(Kind::Concept) => {
                let index = self.child[self.position];
                self.position += 1;
                Ok(vec![Value::Atom(
                    self.tree.source()[self.tree.node()[index].span.clone()].into(),
                )])
            }
            Some(Kind::Context) => Ok(vec![Value::Rule {
                rule: Box::new(self.definition()?),
            }]),
            Some(Kind::Group) => {
                let index = self.child[self.position];
                self.position += 1;
                let program = self.nested(index)?.program()?;
                if program.initial.len() > 1 {
                    return Err(self.failure("a particle group cannot combine separate coherences"));
                }
                let mut particle = program.initial.into_iter().next().unwrap_or_default();
                particle.extend(program.rule.into_iter().map(|rule| Value::Rule {
                    rule: Box::new(rule),
                }));
                Ok(particle)
            }
            _ => Err(self.failure("expected a concept, group, or source context")),
        }
    }
}
