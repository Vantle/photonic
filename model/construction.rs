use crate::context;
use crate::evidence::Evidence;
use crate::failure::Failure;
use crate::fragment::Fragment;
use crate::history::History;
use crate::occurrence::Occurrence;
use crate::structure::{Body, Destination, Input, Output, Particle, Rule, Value};
use std::collections::BTreeSet;

pub struct Construction {
    context: context::Identity,
    history: History,
}

pub struct Inspection {
    pub input: Fragment<Input>,
    pub output: Fragment<Output>,
    context: context::Identity,
    origin: Evidence,
}

impl Construction {
    pub fn new(context: context::Identity, history: History) -> Self {
        Self { context, history }
    }

    fn evidence(&self) -> Evidence {
        Evidence {
            read: BTreeSet::new(),
            context: BTreeSet::from([self.context]),
            history: self.history.clone(),
        }
    }

    fn accept<Item>(&self, value: Fragment<Item>) -> Result<Fragment<Item>, Failure> {
        self.history.permits(&value.evidence.history)?;
        let mut evidence = self.evidence();
        evidence.append(value.evidence);
        Ok(Fragment {
            value: value.value,
            evidence,
        })
    }

    fn collect<Item>(
        &self,
        source: impl IntoIterator<Item = Fragment<Item>>,
    ) -> Result<Fragment<Vec<Item>>, Failure> {
        let mut evidence = self.evidence();
        let mut value = Vec::new();
        for fragment in source {
            self.history.permits(&fragment.evidence.history)?;
            evidence.append(fragment.evidence);
            value.push(fragment.value);
        }
        Ok(Fragment { value, evidence })
    }

    pub fn literal(&self, atom: impl Into<String>) -> Fragment<Value> {
        Fragment {
            value: Value::Atom(atom.into()),
            evidence: self.evidence(),
        }
    }

    pub fn inspect(&self, source: &Occurrence) -> Result<Fragment<Value>, Failure> {
        self.history.permits(&source.history)?;
        let mut evidence = self.evidence();
        evidence.read.insert(source.identity);
        source.value.context(&mut evidence.context);
        Ok(Fragment {
            value: source.value.clone(),
            evidence,
        })
    }

    pub fn particle(
        &self,
        value: impl IntoIterator<Item = Fragment<Value>>,
    ) -> Result<Fragment<Particle>, Failure> {
        Ok(self.collect(value)?.map(Particle::new))
    }

    pub fn input(
        &self,
        particle: impl IntoIterator<Item = Fragment<Particle>>,
    ) -> Result<Fragment<Input>, Failure> {
        Ok(self.collect(particle)?.map(Input::new))
    }

    pub fn destination(
        &self,
        particle: Fragment<Particle>,
        body: Option<Fragment<Body>>,
    ) -> Result<Fragment<Destination>, Failure> {
        let particle = self.accept(particle)?;
        let mut evidence = particle.evidence;
        let body = body
            .map(|body| {
                let body = self.accept(body)?;
                evidence.append(body.evidence);
                Ok(body.value)
            })
            .transpose()?;
        Ok(Fragment {
            value: Destination {
                particle: particle.value,
                body,
            },
            evidence,
        })
    }

    pub fn output(
        &self,
        destination: impl IntoIterator<Item = Fragment<Destination>>,
    ) -> Result<Fragment<Output>, Failure> {
        Ok(self.collect(destination)?.map(Output::new))
    }

    pub fn body(
        &self,
        rule: impl IntoIterator<Item = Fragment<Rule>>,
    ) -> Result<Fragment<Body>, Failure> {
        Ok(self
            .collect(rule)?
            .map(|rule| Body::new(self.context, rule)))
    }

    pub fn definition(&self, value: Fragment<Value>) -> Result<Fragment<Rule>, Failure> {
        let value = self.accept(value)?;
        let Value::Rule(rule) = value.value else {
            return Err(Failure::Rule);
        };
        Ok(Fragment {
            value: *rule,
            evidence: value.evidence,
        })
    }

    pub fn open(&self, value: Fragment<Rule>) -> Result<Inspection, Failure> {
        let value = self.accept(value)?;
        Ok(Inspection {
            input: Fragment {
                value: value.value.input,
                evidence: value.evidence.clone(),
            },
            output: Fragment {
                value: value.value.output,
                evidence: value.evidence.clone(),
            },
            context: value.value.context,
            origin: value.evidence,
        })
    }

    pub fn rule(
        &self,
        input: Fragment<Input>,
        output: Fragment<Output>,
    ) -> Result<Fragment<Value>, Failure> {
        self.close(Inspection {
            input,
            output,
            context: self.context,
            origin: self.evidence(),
        })
    }

    pub fn close(&self, inspection: Inspection) -> Result<Fragment<Value>, Failure> {
        self.history.permits(&inspection.origin.history)?;
        let input = self.accept(inspection.input)?;
        let output = self.accept(inspection.output)?;
        let mut evidence = input.evidence;
        evidence.append(output.evidence);
        evidence.append(inspection.origin);
        evidence.context.insert(inspection.context);
        Ok(Fragment {
            value: Value::Rule(Box::new(Rule {
                input: input.value,
                output: output.value,
                context: inspection.context,
            })),
            evidence,
        })
    }
}
