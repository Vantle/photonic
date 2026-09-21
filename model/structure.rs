use crate::context;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Value {
    Atom(String),
    Rule(Box<Rule>),
}

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Particle(Vec<Value>);

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Input(Vec<Particle>);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Destination {
    pub particle: Particle,
    pub body: Option<Body>,
}

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Output(Vec<Destination>);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Body {
    context: context::Identity,
    rule: Vec<Rule>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Rule {
    pub input: Input,
    pub output: Output,
    pub context: context::Identity,
}

impl Particle {
    pub fn new(mut value: Vec<Value>) -> Self {
        value.sort();
        Self(value)
    }

    pub fn value(&self) -> &[Value] {
        &self.0
    }
}

impl Input {
    pub fn new(mut particle: Vec<Particle>) -> Self {
        particle.sort();
        Self(particle)
    }

    pub fn particle(&self) -> &[Particle] {
        &self.0
    }
}

impl Output {
    pub fn new(mut destination: Vec<Destination>) -> Self {
        destination.sort();
        Self(destination)
    }

    pub fn destination(&self) -> &[Destination] {
        &self.0
    }
}

impl Body {
    pub fn new(context: context::Identity, mut rule: Vec<Rule>) -> Self {
        rule.sort();
        Self { context, rule }
    }

    pub fn context(&self) -> context::Identity {
        self.context
    }

    pub fn rule(&self) -> &[Rule] {
        &self.rule
    }
}

impl Value {
    pub(crate) fn context(&self, result: &mut BTreeSet<context::Identity>) {
        let Self::Rule(rule) = self else {
            return;
        };
        rule.collect(result);
    }
}

impl Rule {
    fn collect(&self, result: &mut BTreeSet<context::Identity>) {
        result.insert(self.context);
        for particle in self.input.particle() {
            for value in particle.value() {
                value.context(result);
            }
        }
        for destination in self.output.destination() {
            for value in destination.particle.value() {
                value.context(result);
            }
            if let Some(body) = &destination.body {
                result.insert(body.context);
                for rule in &body.rule {
                    rule.collect(result);
                }
            }
        }
    }
}
