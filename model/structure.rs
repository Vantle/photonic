use crate::context;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Value<Context = context::Identity, Capture = context::Identity> {
    Atom(String),
    Rule(Box<Rule<Context, Capture>>),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Particle<Context = context::Identity, Capture = context::Identity>(
    Vec<Value<Context, Capture>>,
);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Input<Context = context::Identity, Capture = context::Identity>(
    Vec<Particle<Context, Capture>>,
);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Destination<Context = context::Identity, Capture = context::Identity> {
    pub particle: Particle<Context, Capture>,
    pub body: Option<Body<Context, Capture>>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Output<Context = context::Identity, Capture = context::Identity>(
    Vec<Destination<Context, Capture>>,
);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Body<Context = context::Identity, Capture = context::Identity> {
    pub(crate) context: Context,
    pub(crate) rule: Vec<Rule<context::Reference<Capture>, Capture>>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Rule<Context = context::Identity, Capture = context::Identity> {
    pub input: Input<Context, Capture>,
    pub output: Output<Context, Capture>,
    pub context: Context,
}

impl<Context: Ord, Capture: Ord> Particle<Context, Capture> {
    pub fn new(mut value: Vec<Value<Context, Capture>>) -> Self {
        value.sort();
        Self(value)
    }

    pub fn value(&self) -> &[Value<Context, Capture>] {
        &self.0
    }
}

impl<Context: Ord, Capture: Ord> Input<Context, Capture> {
    pub fn new(mut particle: Vec<Particle<Context, Capture>>) -> Self {
        particle.sort();
        Self(particle)
    }

    pub fn particle(&self) -> &[Particle<Context, Capture>] {
        &self.0
    }
}

impl<Context: Ord, Capture: Ord> Output<Context, Capture> {
    pub fn new(mut destination: Vec<Destination<Context, Capture>>) -> Self {
        destination.sort();
        Self(destination)
    }

    pub fn destination(&self) -> &[Destination<Context, Capture>] {
        &self.0
    }
}

impl<Capture: Clone + Ord> Body<Capture, Capture> {
    pub fn new(context: Capture, rule: Vec<Rule<Capture, Capture>>) -> Self {
        let mut rule = rule
            .into_iter()
            .map(crate::activation::capture)
            .collect::<Vec<_>>();
        rule.sort();
        Self { context, rule }
    }

    pub fn bind(
        context: Capture,
        mut rule: Vec<Rule<context::Reference<Capture>, Capture>>,
    ) -> Result<Self, crate::failure::Failure> {
        for declaration in &rule {
            crate::activation::validate(declaration, 1)?;
        }
        rule.sort();
        Ok(Self { context, rule })
    }

    pub fn activate(
        &self,
        context: Capture,
    ) -> Result<Vec<Rule<Capture, Capture>>, crate::failure::Failure> {
        self.rule
            .iter()
            .cloned()
            .map(|rule| crate::activation::close(rule, context.clone()))
            .collect()
    }
}

impl<Capture: Clone + Ord> Body<context::Reference<Capture>, Capture> {
    pub fn nested(
        context: context::Reference<Capture>,
        mut rule: Vec<Rule<context::Reference<Capture>, Capture>>,
    ) -> Self {
        rule.sort();
        Self { context, rule }
    }
}

impl<Context: Clone, Capture> Body<Context, Capture> {
    pub fn context(&self) -> Context {
        self.context.clone()
    }

    pub fn rule(&self) -> &[Rule<context::Reference<Capture>, Capture>] {
        &self.rule
    }
}

impl<Context, Capture> Value<Context, Capture> {
    pub(crate) fn context(&self, result: &mut BTreeSet<context::Identity>)
    where
        Context: context::Capture,
        Capture: context::Capture,
    {
        let Self::Rule(rule) = self else {
            return;
        };
        rule.collect(result);
    }
}

impl<Context, Capture> Rule<Context, Capture> {
    pub(crate) fn collect(&self, result: &mut BTreeSet<context::Identity>)
    where
        Context: context::Capture,
        Capture: context::Capture,
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

impl<Context, Capture> Default for Particle<Context, Capture> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<Context, Capture> Default for Input<Context, Capture> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<Context, Capture> Default for Output<Context, Capture> {
    fn default() -> Self {
        Self(Vec::new())
    }
}
