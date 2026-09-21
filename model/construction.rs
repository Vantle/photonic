use crate::context;
use crate::evidence::Evidence;
use crate::failure::Failure;
use crate::fragment::Fragment;
use crate::history::History;
use crate::occurrence::Occurrence;
use crate::structure::{Body, Destination, Input, Output, Particle, Rule, Value};
use std::collections::BTreeSet;

#[derive(Clone)]
pub struct Construction<Context = context::Identity> {
    context: Context,
    origin: Evidence,
}

pub struct Inspection<Context = context::Identity> {
    pub input: Fragment<Input<Context>>,
    pub output: Fragment<Output<Context>>,
    context: Context,
    origin: Evidence,
}

impl<Context: Copy + Ord> Construction<Context> {
    pub(crate) fn evidence(&self) -> Evidence {
        self.origin.clone()
    }

    pub(crate) fn permits(&self, evidence: &Evidence) -> Result<(), Failure> {
        self.origin.history.permits(&evidence.history)
    }

    pub(crate) fn accept<Item>(&self, value: Fragment<Item>) -> Result<Fragment<Item>, Failure> {
        self.origin.history.permits(&value.evidence.history)?;
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
            self.origin.history.permits(&fragment.evidence.history)?;
            evidence.append(fragment.evidence);
            value.push(fragment.value);
        }
        Ok(Fragment { value, evidence })
    }

    pub fn literal(&self, atom: impl Into<String>) -> Fragment<Value<Context>> {
        Fragment {
            value: Value::Atom(atom.into()),
            evidence: self.evidence(),
        }
    }

    pub fn particle(
        &self,
        value: impl IntoIterator<Item = Fragment<Value<Context>>>,
    ) -> Result<Fragment<Particle<Context>>, Failure> {
        Ok(self.collect(value)?.map(Particle::new))
    }

    pub fn input(
        &self,
        particle: impl IntoIterator<Item = Fragment<Particle<Context>>>,
    ) -> Result<Fragment<Input<Context>>, Failure> {
        Ok(self.collect(particle)?.map(Input::new))
    }

    pub fn destination(
        &self,
        particle: Fragment<Particle<Context>>,
        body: Option<Fragment<Body<Context>>>,
    ) -> Result<Fragment<Destination<Context>>, Failure> {
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
        destination: impl IntoIterator<Item = Fragment<Destination<Context>>>,
    ) -> Result<Fragment<Output<Context>>, Failure> {
        Ok(self.collect(destination)?.map(Output::new))
    }

    pub fn definition(
        &self,
        value: Fragment<Value<Context>>,
    ) -> Result<Fragment<Rule<Context>>, Failure> {
        let value = self.accept(value)?;
        let Value::Rule(rule) = value.value else {
            return Err(Failure::Rule);
        };
        Ok(Fragment {
            value: *rule,
            evidence: value.evidence,
        })
    }

    pub fn open(&self, value: Fragment<Rule<Context>>) -> Result<Inspection<Context>, Failure> {
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
        input: Fragment<Input<Context>>,
        output: Fragment<Output<Context>>,
    ) -> Result<Fragment<Value<Context>>, Failure> {
        self.close(Inspection {
            input,
            output,
            context: self.context,
            origin: self.evidence(),
        })
    }

    pub fn close(
        &self,
        inspection: Inspection<Context>,
    ) -> Result<Fragment<Value<Context>>, Failure> {
        self.origin.history.permits(&inspection.origin.history)?;
        let input = self.accept(inspection.input)?;
        let output = self.accept(inspection.output)?;
        let mut evidence = input.evidence;
        evidence.append(output.evidence);
        evidence.append(inspection.origin);
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

impl Construction {
    pub fn new(context: context::Identity, history: History) -> Self {
        Self {
            context,
            origin: Evidence {
                read: BTreeSet::new(),
                context: BTreeSet::from([context]),
                history,
            },
        }
    }

    pub fn defer(&self) -> Construction<context::Reference> {
        Construction {
            context: context::Reference::Captured(self.context),
            origin: self.evidence(),
        }
    }

    pub fn seal<Item: crate::activation::Seal>(
        &self,
        value: Fragment<Item>,
    ) -> Result<Fragment<Item::Target>, Failure> {
        let value = self.accept(value)?;
        Ok(Fragment {
            value: value.value.seal()?,
            evidence: value.evidence,
        })
    }

    pub fn inspect(&self, source: &Occurrence) -> Result<Fragment<Value>, Failure> {
        self.origin.history.permits(&source.history)?;
        let mut evidence = self.evidence();
        evidence.read.insert(source.identity);
        source.value.context(&mut evidence.context);
        Ok(Fragment {
            value: source.value.clone(),
            evidence,
        })
    }

    pub fn body(
        &self,
        rule: impl IntoIterator<Item = Fragment<Rule>>,
    ) -> Result<Fragment<Body>, Failure> {
        Ok(self
            .collect(rule)?
            .map(|rule| Body::new(self.context, rule)))
    }
}

impl Construction<context::Reference> {
    pub fn local(&self) -> Self {
        Self {
            context: context::Reference::Local(0),
            origin: self.evidence(),
        }
    }

    pub fn capture<Item: crate::activation::Capture>(
        &self,
        value: Fragment<Item>,
    ) -> Result<Fragment<Item::Target>, Failure> {
        Ok(self.accept(value)?.map(Item::capture))
    }

    pub fn body(
        &self,
        rule: impl IntoIterator<Item = Fragment<Rule<context::Reference>>>,
    ) -> Result<Fragment<Body<context::Reference>>, Failure> {
        Ok(self
            .collect(rule)?
            .map(|rule| Body::nested(self.context, rule)))
    }
}
