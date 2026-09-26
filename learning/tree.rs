use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::scope::Scope;
use code::value::Value;

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
        value(particle, depth, index, result);
    }
    for output in rule.output() {
        match output {
            Output::Particle(particle) => value(particle, depth, index, result),
            Output::Scope(scope) => enclose(scope, depth, index, result),
        }
    }
}

fn value<'program>(
    particle: &'program Particle,
    depth: usize,
    parent: usize,
    result: &mut Vec<Node<'program>>,
) {
    for value in particle.value() {
        if let Value::Rule(nested) = value {
            visit(nested, depth + 1, Place::Value { parent }, result);
        }
    }
}

fn enclose<'program>(
    scope: &'program Scope,
    depth: usize,
    parent: usize,
    result: &mut Vec<Node<'program>>,
) {
    for particle in scope.coherence() {
        value(particle, depth, parent, result);
    }
    for nested in scope.rule() {
        visit(nested, depth + 1, Place::Body { parent }, result);
    }
    for nested in scope.scope() {
        enclose(nested, depth, parent, result);
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
            .flat_map(|output| match output {
                Output::Particle(particle) => vec![Output::Particle(self.particle(particle))],
                Output::Scope(scope) => self.scope(scope),
            })
            .collect();
        vec![Rule::new(input, output)]
    }

    fn scope(&mut self, scope: &Scope) -> Vec<Output> {
        let mut member = scope
            .coherence()
            .iter()
            .map(|particle| Output::Particle(self.particle(particle)))
            .collect::<Vec<_>>();
        let rule = self.list(scope.rule());
        for nested in scope.scope() {
            member.extend(self.scope(nested));
        }
        Output::group(member, rule)
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
    Program::new(rewrite.list(program.rule()), program.scope().to_vec())
}
