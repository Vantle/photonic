use crate::output::Output;
use crate::particle::Particle;
use crate::program::Program;
use crate::rule::Rule;
use crate::value::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Place {
    Program,
    Value { parent: usize },
    Body { parent: usize },
}

#[derive(Clone, Copy, Debug)]
pub struct Node<'program> {
    pub rule: &'program Rule,
    pub depth: usize,
    pub place: Place,
}

fn visit<'program>(
    rule: &'program Rule,
    depth: usize,
    place: Place,
    result: &mut Vec<Node<'program>>,
) {
    let index = result.len();
    result.push(Node { rule, depth, place });
    for particle in rule.input() {
        for value in particle.value() {
            if let Value::Rule(nested) = value {
                visit(nested, depth + 1, Place::Value { parent: index }, result);
            }
        }
    }
    for output in rule.output() {
        for value in output.particle().value() {
            if let Value::Rule(nested) = value {
                visit(nested, depth + 1, Place::Value { parent: index }, result);
            }
        }
        for nested in output.body().unwrap_or_default() {
            visit(nested, depth + 1, Place::Body { parent: index }, result);
        }
    }
}

pub fn walk(program: &Program) -> Vec<Node<'_>> {
    let mut result = Vec::new();
    for rule in program.rule() {
        visit(rule, 0, Place::Program, &mut result);
    }
    result
}

struct Rewrite<Change> {
    target: usize,
    counter: usize,
    change: Option<Change>,
}

impl<Change: FnOnce(&Rule) -> Vec<Rule>> Rewrite<Change> {
    fn rule(&mut self, rule: &Rule) -> Vec<Rule> {
        let index = self.counter;
        self.counter += 1;
        if index == self.target {
            let change = self.change.take().expect("a rewrite applies once");
            return change(rule);
        }
        let input = rule
            .input()
            .iter()
            .map(|particle| self.particle(particle))
            .collect();
        let output = rule
            .output()
            .iter()
            .map(|output| {
                let particle = self.particle(output.particle());
                let body = output.body().map(|body| self.list(body));
                Output::new(particle, body)
            })
            .collect();
        vec![Rule::new(input, output)]
    }

    fn particle(&mut self, particle: &Particle) -> Particle {
        Particle::from(
            particle
                .value()
                .iter()
                .flat_map(|value| match value {
                    Value::Atom(atom) => vec![Value::Atom(*atom)],
                    Value::Rule(rule) => self
                        .rule(rule)
                        .into_iter()
                        .map(|rule| Value::Rule(Box::new(rule)))
                        .collect(),
                })
                .collect::<Vec<_>>(),
        )
    }

    fn list(&mut self, rule: &[Rule]) -> Vec<Rule> {
        rule.iter().flat_map(|rule| self.rule(rule)).collect()
    }
}

pub fn transform(
    program: &Program,
    target: usize,
    change: impl FnOnce(&Rule) -> Vec<Rule>,
) -> Program {
    let mut rewrite = Rewrite {
        target,
        counter: 0,
        change: Some(change),
    };
    Program::from(rewrite.list(program.rule()))
}
