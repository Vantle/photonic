use std::ops::Range;

use miette::NamedSource;

use crate::failure::Failure;
use crate::source::{Definition, Output, Program, Value};
use crate::syntax::{Kind, Tree};

const BUDGET: usize = 1_000_000;
// A rule's name repeats the text of every rule nested inside it, so names alone can cost the
// nesting depth times the source; the library's deepest programs spend about four times their
// source on names, and this keeps any program within a constant multiple of what it reads.
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
    Reader {
        tree: &tree,
        budget: allowance(source),
    }
    .program()
}

fn allowance(source: &str) -> usize {
    BUDGET.saturating_add(source.len().saturating_mul(RATIO))
}

impl<'tree> Reader<'tree, '_> {
    fn child(&self, index: usize) -> &'tree [usize] {
        self.tree.child(index)
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
        Ok(assemble(self.place(self.child(0)[0])?))
    }

    fn split(&self, term: usize) -> (Vec<usize>, Vec<usize>) {
        self.child(term)
            .iter()
            .partition(|&&node| self.kind(node) == Kind::Bracket)
    }

    fn place(&mut self, list: usize) -> Result<Vec<Member>, Failure> {
        let mut result = Vec::new();
        for &term in self.child(list) {
            let (bracket, part) = self.split(term);
            if bracket.is_empty() {
                result.extend(self.produce(&part)?);
                continue;
            }
            result.push(Member::Rule(self.rule(term, &bracket, &part)?));
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
        let mut member = self.place(self.child(index)[0])?;
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
        let mut result = vec![Vec::new()];
        for &part in part {
            let value = self.factor(part)?;
            result = crate::expansion::combine(result, value, &mut self.budget)
                .ok_or_else(|| self.expansion(self.span(part)))?;
        }
        Ok(result)
    }

    fn factor(&mut self, index: usize) -> Result<Vec<Vec<Value>>, Failure> {
        if self.kind(index) == Kind::Atom {
            return Ok(vec![vec![Value::Atom(
                self.tree.source()[self.span(index)].into(),
            )]]);
        }
        let particle = self.particle(self.child(index)[0])?;
        if particle.is_empty() {
            return Ok(vec![Vec::new()]);
        }
        Ok(particle)
    }

    fn particle(&mut self, list: usize) -> Result<Vec<Vec<Value>>, Failure> {
        let mut result = Vec::new();
        for &term in self.child(list) {
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
        let span = self.span(term);
        let mut input = vec![Vec::new()];
        for &node in bracket {
            let pattern = self.particle(self.child(node)[0])?;
            input = crate::expansion::combine(input, pattern, &mut self.budget)
                .ok_or_else(|| self.expansion(span.clone()))?;
        }
        let mut output = Vec::new();
        for member in self.produce(part)? {
            output.push(match member {
                Member::Particle(particle) => Output::Particle(particle),
                Member::Scope(program) => Output::Scope(program),
                Member::Rule(_) => unreachable!("a group that lists a rule is a scope"),
            });
        }
        let name = self.tree.source()[span.clone()].trim_end().to_owned();
        self.budget = self
            .budget
            .checked_sub(name.len())
            .ok_or_else(|| self.expansion(span))?;
        Ok(Definition {
            name,
            input,
            output,
        })
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
        match member {
            Member::Particle(particle) => program.initial.push(particle),
            Member::Rule(rule) => program.rule.push(rule),
            Member::Scope(scope) => program.scope.push(scope),
        }
    }
    program
}
