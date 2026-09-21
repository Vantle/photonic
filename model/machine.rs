use crate::construction::Construction;
use crate::environment::Environment;
use crate::failure::Failure;
use crate::fragment::Fragment;
use crate::{structure, template};
use std::task::Poll;

enum Product {
    Value(Fragment<structure::Value>),
    Particle(Fragment<structure::Particle>),
    Input(Fragment<structure::Input>),
    Destination(Fragment<structure::Destination>),
    Output(Fragment<structure::Output>),
    Body(Fragment<structure::Body>),
}

enum Task<'a> {
    Value(&'a template::Value),
    Particle(&'a template::Particle),
    Input(&'a template::Input),
    Destination(&'a template::Destination),
    Output(&'a template::Output),
    Body(&'a template::Body),
    Rule(&'a template::Rule),
    Collect(&'a [template::Value], Vec<Fragment<structure::Value>>),
    Gather(&'a [template::Particle], Vec<Fragment<structure::Particle>>),
    Assemble(
        &'a [template::Destination],
        Vec<Fragment<structure::Destination>>,
    ),
    Declare(&'a [template::Rule], Vec<Fragment<structure::Rule>>),
    Attach(&'a template::Destination),
    Enter(Fragment<structure::Particle>),
    Prepare(&'a template::Rule),
    Close(Fragment<structure::Input>),
}

pub struct Machine<'a> {
    construction: &'a Construction,
    environment: &'a Environment,
    task: Vec<Task<'a>>,
    result: Option<Product>,
    outcome: Option<Result<Fragment<structure::Value>, Failure>>,
    work: usize,
}

impl<'a> Machine<'a> {
    pub fn new(
        value: &'a template::Value,
        construction: &'a Construction,
        environment: &'a Environment,
    ) -> Self {
        Self {
            construction,
            environment,
            task: vec![Task::Value(value)],
            result: None,
            outcome: None,
            work: 0,
        }
    }

    pub fn work(&self) -> usize {
        self.work
    }

    pub fn run(&mut self, budget: usize) -> Poll<Result<Fragment<structure::Value>, Failure>> {
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
                let Some(Product::Value(value)) = self.result.take() else {
                    unreachable!();
                };
                self.outcome = Some(Ok(value));
            }
        }
        self.outcome.clone().map_or(Poll::Pending, Poll::Ready)
    }

    fn advance(&mut self, task: Task<'a>) -> Result<(), Failure> {
        match task {
            Task::Value(value) => match value {
                template::Value::Rule(rule) => self.task.push(Task::Rule(rule)),
                _ => {
                    self.result = Some(Product::Value(
                        value.instantiate(self.construction, self.environment)?,
                    ));
                }
            },
            Task::Particle(particle) => match particle {
                template::Particle::Build(value) => {
                    self.task.push(Task::Collect(value, Vec::new()));
                }
                _ => {
                    self.result = Some(Product::Particle(
                        particle.instantiate(self.construction, self.environment)?,
                    ));
                }
            },
            Task::Input(input) => match input {
                template::Input::Build(particle) => {
                    self.task.push(Task::Gather(particle, Vec::new()));
                }
                _ => {
                    self.result = Some(Product::Input(
                        input.instantiate(self.construction, self.environment)?,
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
                        output.instantiate(self.construction, self.environment)?,
                    ));
                }
            },
            Task::Body(body) => match body {
                template::Body::Build(rule) => {
                    self.task.push(Task::Declare(rule, Vec::new()));
                }
                _ => {
                    self.result = Some(Product::Body(
                        body.instantiate(self.construction, self.environment)?,
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
            Task::Declare(source, mut value) => {
                if let Some(result) = self.result.take() {
                    let Product::Value(result) = result else {
                        unreachable!();
                    };
                    value.push(self.construction.definition(result)?);
                }
                if let Some((first, rest)) = source.split_first() {
                    self.task.push(Task::Declare(rest, value));
                    self.task.push(Task::Rule(first));
                } else {
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
