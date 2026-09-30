use std::ops::Range;

use miette::NamedSource;

use crate::failure::Failure;
use crate::source::{Definition, Library, Output, Program, Value};
use crate::syntax::{Kind, Tree};

const BUDGET: usize = 1_000_000;
// A large program distributes many small joins, each into a few times its text, so the budget
// grows with the source and only a term that multiplies itself runs out of it.
const RATIO: usize = 8;

enum Member {
    Particle(Vec<Value>),
    Rule(Definition),
    Scope(Program),
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
    Reader::new(&crate::parser::parse(source)?).program()
}

pub fn library(source: &str) -> Result<Library, Failure> {
    Reader::new(&crate::parser::parse(source)?).library()
}

fn allowance(source: &str) -> usize {
    BUDGET.saturating_add(source.len().saturating_mul(RATIO))
}

impl<'tree, 'source> Reader<'tree, 'source> {
    fn new(tree: &'tree Tree<'source>) -> Self {
        Self {
            tree,
            budget: allowance(tree.source()),
        }
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

    fn library(&mut self) -> Result<Library, Failure> {
        let mut rule = Vec::new();
        for term in self.tree.child(self.first(0)) {
            for member in self.term(term)? {
                let Member::Rule(definition) = member else {
                    let span = self.span(term);
                    return Err(Failure::Library {
                        span: (span.start, span.len()).into(),
                    });
                };
                rule.push(definition);
            }
        }
        Ok(Library { rule })
    }

    fn split(&self, term: usize) -> (Vec<usize>, Vec<usize>) {
        self.tree
            .child(term)
            .partition(|&node| self.kind(node) == Kind::Bracket)
    }

    fn term(&mut self, term: usize) -> Result<Vec<Member>, Failure> {
        let (bracket, part) = self.split(term);
        if bracket.is_empty() {
            return self.produce(&part);
        }
        Ok(vec![Member::Rule(self.rule(term, &bracket, &part)?)])
    }

    fn place(&mut self, list: usize) -> Result<Vec<Member>, Failure> {
        let mut result = Vec::new();
        for term in self.tree.child(list) {
            result.extend(self.term(term)?);
        }
        Ok(result)
    }

    fn produce(&mut self, part: &[usize]) -> Result<Vec<Member>, Failure> {
        match part {
            [] => Ok(Vec::new()),
            &[group] if self.kind(group) == Kind::Group => self.group(group),
            _ => Ok(self.join(part)?.into_iter().map(Member::Particle).collect()),
        }
    }

    fn group(&mut self, index: usize) -> Result<Vec<Member>, Failure> {
        let mut member = self.place(self.first(index))?;
        // A group always holds a place, so () is the empty particle and the remainder of the rule
        // that opens a scope always has a coherence to enter.
        if !member
            .iter()
            .any(|member| matches!(member, Member::Particle(_) | Member::Scope(_)))
        {
            member.push(Member::Particle(Vec::new()));
        }
        if !member
            .iter()
            .any(|member| matches!(member, Member::Rule(_)))
        {
            return Ok(member);
        }
        Ok(vec![Member::Scope(assemble(member))])
    }

    fn join(&mut self, part: &[usize]) -> Result<Vec<Vec<Value>>, Failure> {
        let mut choice = Vec::<Vec<Vec<Value>>>::new();
        for &index in part {
            let alternative = self.factor(index)?;
            match (choice.last_mut(), alternative.len()) {
                (Some(last), 1) if last.len() == 1 => {
                    last[0].extend(alternative.into_iter().flatten());
                }
                _ => choice.push(alternative),
            }
        }
        crate::expansion::product(choice, &mut self.budget).ok_or_else(|| {
            let start = part.first().map_or(0, |&first| self.span(first).start);
            let end = part.last().map_or(start, |&last| self.span(last).end);
            self.expansion(start..end)
        })
    }

    fn factor(&mut self, index: usize) -> Result<Vec<Vec<Value>>, Failure> {
        if self.kind(index) == Kind::Atom {
            return Ok(vec![vec![Value::Atom(
                self.tree.source()[self.span(index)].into(),
            )]]);
        }
        let particle = self.particle(self.first(index))?;
        if particle.is_empty() {
            return Ok(vec![Vec::new()]);
        }
        Ok(particle)
    }

    fn particle(&mut self, list: usize) -> Result<Vec<Vec<Value>>, Failure> {
        let mut result = Vec::new();
        for term in self.tree.child(list) {
            let (bracket, part) = self.split(term);
            if !bracket.is_empty() {
                result.push(vec![value(self.rule(term, &bracket, &part)?)]);
                continue;
            }
            if !part.is_empty() {
                result.extend(self.join(&part)?);
            }
        }
        Ok(result)
    }

    fn rule(
        &mut self,
        term: usize,
        bracket: &[usize],
        part: &[usize],
    ) -> Result<Definition, Failure> {
        let pattern = bracket
            .iter()
            .map(|&node| self.particle(self.first(node)))
            .collect::<Result<_, _>>()?;
        let input = crate::expansion::product(pattern, &mut self.budget)
            .ok_or_else(|| self.expansion(self.span(term)))?;
        let output = self
            .produce(part)?
            .into_iter()
            .map(|member| match member {
                Member::Particle(particle) => Output::Particle(particle),
                Member::Scope(program) => Output::Scope(program),
                Member::Rule(_) => unreachable!("a group that lists a rule is a scope"),
            })
            .collect();
        Ok(Definition { input, output })
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
        Member::Rule(rule) => program.rule.push(rule),
        Member::Scope(scope) => program.scope.push(scope),
    }
}
