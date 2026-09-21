use crate::context;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Value<Context = context::Identity> {
    Atom(String),
    Rule(Box<Rule<Context>>),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Particle<Context = context::Identity>(Vec<Value<Context>>);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Input<Context = context::Identity>(Vec<Particle<Context>>);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Destination<Context = context::Identity> {
    pub particle: Particle<Context>,
    pub body: Option<Body<Context>>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Output<Context = context::Identity>(Vec<Destination<Context>>);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Body<Context = context::Identity> {
    pub(crate) context: Context,
    pub(crate) rule: Vec<Rule<context::Reference>>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Rule<Context = context::Identity> {
    pub input: Input<Context>,
    pub output: Output<Context>,
    pub context: Context,
}

impl<Context: Ord> Particle<Context> {
    pub fn new(mut value: Vec<Value<Context>>) -> Self {
        value.sort();
        Self(value)
    }

    pub fn value(&self) -> &[Value<Context>] {
        &self.0
    }
}

impl<Context: Ord> Input<Context> {
    pub fn new(mut particle: Vec<Particle<Context>>) -> Self {
        particle.sort();
        Self(particle)
    }

    pub fn particle(&self) -> &[Particle<Context>] {
        &self.0
    }
}

impl<Context: Ord> Output<Context> {
    pub fn new(mut destination: Vec<Destination<Context>>) -> Self {
        destination.sort();
        Self(destination)
    }

    pub fn destination(&self) -> &[Destination<Context>] {
        &self.0
    }
}

impl Body {
    pub fn new(context: context::Identity, rule: Vec<Rule>) -> Self {
        let mut rule = rule
            .into_iter()
            .map(crate::activation::capture)
            .collect::<Vec<_>>();
        rule.sort();
        Self { context, rule }
    }

    pub fn bind(
        context: context::Identity,
        mut rule: Vec<Rule<context::Reference>>,
    ) -> Result<Self, crate::failure::Failure> {
        for declaration in &rule {
            crate::activation::validate(declaration, 1)?;
        }
        rule.sort();
        Ok(Self { context, rule })
    }

    pub fn activate(
        &self,
        context: context::Identity,
    ) -> Result<Vec<Rule>, crate::failure::Failure> {
        self.rule
            .iter()
            .cloned()
            .map(|rule| crate::activation::close(rule, context))
            .collect()
    }
}

impl Body<context::Reference> {
    pub fn nested(context: context::Reference, mut rule: Vec<Rule<context::Reference>>) -> Self {
        rule.sort();
        Self { context, rule }
    }
}

impl<Context: Copy> Body<Context> {
    pub fn context(&self) -> Context {
        self.context
    }

    pub fn rule(&self) -> &[Rule<context::Reference>] {
        &self.rule
    }
}

impl<Context> Value<Context> {
    pub(crate) fn context(&self, result: &mut BTreeSet<context::Identity>)
    where
        Context: context::Capture,
    {
        let Self::Rule(rule) = self else {
            return;
        };
        rule.collect(result);
    }
}

impl<Context> Rule<Context> {
    pub(crate) fn collect(&self, result: &mut BTreeSet<context::Identity>)
    where
        Context: context::Capture,
    {
        self.context.collect(result);
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
                body.context.collect(result);
                for rule in &body.rule {
                    rule.collect(result);
                }
            }
        }
    }
}

impl<Context> Default for Particle<Context> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<Context> Default for Input<Context> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<Context> Default for Output<Context> {
    fn default() -> Self {
        Self(Vec::new())
    }
}
