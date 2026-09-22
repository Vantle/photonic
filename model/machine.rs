use crate::construction::Construction;
use crate::context::Reference;
use crate::environment::Environment;
use crate::evidence::Evidence;
use crate::failure::Failure;
use crate::fragment::Fragment;
use crate::{structure, template};
use std::task::Poll;

enum Product<Capture> {
    Value(Fragment<structure::Value<Reference<Capture>, Capture>>),
    Particle(Fragment<structure::Particle<Reference<Capture>, Capture>>),
    Input(Fragment<structure::Input<Reference<Capture>, Capture>>),
    Destination(Fragment<structure::Destination<Reference<Capture>, Capture>>),
    Output(Fragment<structure::Output<Reference<Capture>, Capture>>),
    Body(Fragment<structure::Body<Reference<Capture>, Capture>>),
}

enum Task<'a, Capture> {
    Value(&'a template::Value<Capture>),
    Particle(&'a template::Particle<Capture>),
    Input(&'a template::Input<Capture>),
    Destination(&'a template::Destination<Capture>),
    Output(&'a template::Output<Capture>),
    Body(&'a template::Body<Capture>),
    Rule(&'a template::Rule<Capture>),
    Collect(
        &'a [template::Value<Capture>],
        Vec<Fragment<structure::Value<Reference<Capture>, Capture>>>,
    ),
    Gather(
        &'a [template::Particle<Capture>],
        Vec<Fragment<structure::Particle<Reference<Capture>, Capture>>>,
    ),
    Assemble(
        &'a [template::Destination<Capture>],
        Vec<Fragment<structure::Destination<Reference<Capture>, Capture>>>,
    ),
    Declare(
        &'a [template::Rule<Capture>],
        Vec<Fragment<structure::Rule<Reference<Capture>, Capture>>>,
        Construction<Reference<Capture>, Capture>,
    ),
    Attach(&'a template::Destination<Capture>),
    Enter(Fragment<structure::Particle<Reference<Capture>, Capture>>),
    Prepare(&'a template::Rule<Capture>),
    Close(Fragment<structure::Input<Reference<Capture>, Capture>>),
}

pub struct Machine<'a, Capture = crate::context::Identity> {
    construction: Construction<Reference<Capture>, Capture>,
    sealing: &'a Construction<Capture, Capture>,
    environment: &'a Environment<Capture>,
    task: Vec<Task<'a, Capture>>,
    result: Option<Product<Capture>>,
    outcome: Option<Result<Fragment<structure::Value<Capture, Capture>>, Failure>>,
    origin: Option<Evidence>,
    work: usize,
}

impl<'a, Capture: Clone + Ord> Machine<'a, Capture> {
    pub fn new(
        value: &'a template::Value<Capture>,
        construction: &'a Construction<Capture, Capture>,
        environment: &'a Environment<Capture>,
    ) -> Self {
        Self {
            construction: construction.defer(),
            sealing: construction,
            environment,
            task: vec![Task::Value(value)],
            result: None,
            outcome: None,
            origin: None,
            work: 0,
        }
    }

    pub(crate) fn supported(
        value: &'a template::Value<Capture>,
        construction: &'a Construction<Capture, Capture>,
        environment: &'a Environment<Capture>,
        origin: Evidence,
    ) -> Self {
        let mut machine = Self::new(value, construction, environment);
        machine.origin = Some(origin);
        machine
    }

    pub fn work(&self) -> usize {
        self.work
    }

    pub fn run(
        &mut self,
        budget: usize,
    ) -> Poll<Result<Fragment<structure::Value<Capture, Capture>>, Failure>> {
        for _ in 0..budget {
            if self.outcome.is_some() {
                break;
            }
            let Some(work) = self.work.checked_add(1) else {
                self.outcome = Some(Err(Failure::Capacity));
                break;
            };
            self.work = work;
            let task = self.task.pop().unwrap();
            if let Err(failure) = self.advance(task) {
                self.task.clear();
                self.result = None;
                self.outcome = Some(Err(failure));
                break;
            }
            if self.task.is_empty() {
                let Some(Product::Value(mut value)) = self.result.take() else {
                    unreachable!();
                };
                let result = if let Some(origin) = self.origin.take() {
                    value.evidence.append(origin).map(|_| value)
                } else {
                    Ok(value)
                };
                self.outcome = Some(result.and_then(|value| self.sealing.seal(value)));
            }
        }
        self.outcome.clone().map_or(Poll::Pending, Poll::Ready)
    }

    fn advance(&mut self, task: Task<'a, Capture>) -> Result<(), Failure> {
        match task {
            Task::Value(value) => match value {
                template::Value::Rule(rule) => self.task.push(Task::Rule(rule)),
                _ => {
                    self.result = Some(Product::Value(
                        value.defer(&self.construction, self.environment)?,
                    ));
                }
            },
            Task::Particle(particle) => match particle {
                template::Particle::Build(value) => {
                    self.task.push(Task::Collect(value, Vec::new()));
                }
                _ => {
                    self.result = Some(Product::Particle(
                        particle.defer(&self.construction, self.environment)?,
                    ));
                }
            },
            Task::Input(input) => match input {
                template::Input::Build(particle) => {
                    self.task.push(Task::Gather(particle, Vec::new()));
                }
                _ => {
                    self.result = Some(Product::Input(
                        input.defer(&self.construction, self.environment)?,
                    ));
                }
            },
            Task::Destination(destination) => {
                self.task.push(Task::Attach(destination));
                self.task.push(Task::Particle(&destination.particle));
            }
            Task::Output(output) => match output {
                template::Output::Build(destination) => {
                    self.task.push(Task::Assemble(destination, Vec::new()));
                }
                _ => {
                    self.result = Some(Product::Output(
                        output.defer(&self.construction, self.environment)?,
                    ));
                }
            },
            Task::Body(body) => match body {
                template::Body::Build(rule) => {
                    let local = self.construction.local();
                    let outer = std::mem::replace(&mut self.construction, local);
                    self.task.push(Task::Declare(rule, Vec::new(), outer));
                }
                _ => {
                    self.result = Some(Product::Body(
                        body.defer(&self.construction, self.environment)?,
                    ));
                }
            },
            Task::Rule(rule) => {
                self.task.push(Task::Prepare(rule));
                self.task.push(Task::Input(&rule.input));
            }
            Task::Collect(source, mut value) => {
                if let Some(result) = self.result.take() {
                    let Product::Value(result) = result else {
                        unreachable!();
                    };
                    value.push(result);
                }
                if let Some((first, rest)) = source.split_first() {
                    self.task.push(Task::Collect(rest, value));
                    self.task.push(Task::Value(first));
                } else {
                    self.result = Some(Product::Particle(self.construction.particle(value)?));
                }
            }
            Task::Gather(source, mut value) => {
                if let Some(result) = self.result.take() {
                    let Product::Particle(result) = result else {
                        unreachable!();
                    };
                    value.push(result);
                }
                if let Some((first, rest)) = source.split_first() {
                    self.task.push(Task::Gather(rest, value));
                    self.task.push(Task::Particle(first));
                } else {
                    self.result = Some(Product::Input(self.construction.input(value)?));
                }
            }
            Task::Assemble(source, mut value) => {
                if let Some(result) = self.result.take() {
                    let Product::Destination(result) = result else {
                        unreachable!();
                    };
                    value.push(result);
                }
                if let Some((first, rest)) = source.split_first() {
                    self.task.push(Task::Assemble(rest, value));
                    self.task.push(Task::Destination(first));
                } else {
                    self.result = Some(Product::Output(self.construction.output(value)?));
                }
            }
            Task::Declare(source, mut value, outer) => {
                if let Some(result) = self.result.take() {
                    let Product::Value(result) = result else {
                        unreachable!();
                    };
                    value.push(self.construction.definition(result)?);
                }
                if let Some((first, rest)) = source.split_first() {
                    self.task.push(Task::Declare(rest, value, outer));
                    self.task.push(Task::Rule(first));
                } else {
                    self.construction = outer;
                    self.result = Some(Product::Body(self.construction.body(value)?));
                }
            }
            Task::Attach(destination) => {
                let Some(Product::Particle(particle)) = self.result.take() else {
                    unreachable!();
                };
                if let Some(body) = &destination.body {
                    self.task.push(Task::Enter(particle));
                    self.task.push(Task::Body(body));
                } else {
                    self.result = Some(Product::Destination(
                        self.construction.destination(particle, None)?,
                    ));
                }
            }
            Task::Enter(particle) => {
                let Some(Product::Body(body)) = self.result.take() else {
                    unreachable!();
                };
                self.result = Some(Product::Destination(
                    self.construction.destination(particle, Some(body))?,
                ));
            }
            Task::Prepare(rule) => {
                let Some(Product::Input(input)) = self.result.take() else {
                    unreachable!();
                };
                self.task.push(Task::Close(input));
                self.task.push(Task::Output(&rule.output));
            }
            Task::Close(input) => {
                let Some(Product::Output(output)) = self.result.take() else {
                    unreachable!();
                };
                self.result = Some(Product::Value(self.construction.rule(input, output)?));
            }
        }
        Ok(())
    }
}
