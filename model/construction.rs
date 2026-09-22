use crate::context;
use crate::evidence::Evidence;
use crate::failure::Failure;
use crate::fragment::Fragment;
use crate::history::History;
use crate::occurrence::Occurrence;
use crate::structure::{Body, Destination, Input, Output, Particle, Rule, Value};
use std::collections::BTreeSet;

#[derive(Clone)]
pub struct Construction<Context = context::Identity, Capture = context::Identity> {
    pub(crate) context: Context,
    pub(crate) origin: Evidence,
    pub(crate) capture: std::marker::PhantomData<Capture>,
}

pub struct Inspection<Context = context::Identity, Capture = context::Identity> {
    pub input: Fragment<Input<Context, Capture>>,
    pub output: Fragment<Output<Context, Capture>>,
    context: Context,
    origin: Evidence,
}

impl<Context: Clone + Ord, Capture: Clone + Ord> Construction<Context, Capture> {
    pub(crate) fn evidence(&self) -> Evidence {
        self.origin.clone()
    }

    pub(crate) fn permits(&self, evidence: &Evidence) -> Result<(), Failure> {
        self.origin.permits(evidence)
    }

    pub(crate) fn accept<Item>(&self, value: Fragment<Item>) -> Result<Fragment<Item>, Failure> {
        self.origin.history.permits(&value.evidence.history)?;
        let mut evidence = self.evidence();
        evidence.append(value.evidence)?;
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
            evidence.append(fragment.evidence)?;
            value.push(fragment.value);
        }
        Ok(Fragment { value, evidence })
    }

    pub fn literal(&self, atom: impl Into<String>) -> Fragment<Value<Context, Capture>> {
        Fragment {
            value: Value::Atom(atom.into()),
            evidence: self.evidence(),
        }
    }

    pub fn particle(
        &self,
        value: impl IntoIterator<Item = Fragment<Value<Context, Capture>>>,
    ) -> Result<Fragment<Particle<Context, Capture>>, Failure> {
        Ok(self.collect(value)?.map(Particle::new))
    }

    pub fn input(
        &self,
        particle: impl IntoIterator<Item = Fragment<Particle<Context, Capture>>>,
    ) -> Result<Fragment<Input<Context, Capture>>, Failure> {
        Ok(self.collect(particle)?.map(Input::new))
    }

    pub fn destination(
        &self,
        particle: Fragment<Particle<Context, Capture>>,
        body: Option<Fragment<Body<Context, Capture>>>,
    ) -> Result<Fragment<Destination<Context, Capture>>, Failure> {
        let particle = self.accept(particle)?;
        let mut evidence = particle.evidence;
        let body = body
            .map(|body| {
                let body = self.accept(body)?;
                evidence.append(body.evidence)?;
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
        destination: impl IntoIterator<Item = Fragment<Destination<Context, Capture>>>,
    ) -> Result<Fragment<Output<Context, Capture>>, Failure> {
        Ok(self.collect(destination)?.map(Output::new))
    }

    pub fn definition(
        &self,
        value: Fragment<Value<Context, Capture>>,
    ) -> Result<Fragment<Rule<Context, Capture>>, Failure> {
        let value = self.accept(value)?;
        let Value::Rule(rule) = value.value else {
            return Err(Failure::Rule);
        };
        Ok(Fragment {
            value: *rule,
            evidence: value.evidence,
        })
    }

    pub fn open(
        &self,
        value: Fragment<Rule<Context, Capture>>,
    ) -> Result<Inspection<Context, Capture>, Failure> {
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
        input: Fragment<Input<Context, Capture>>,
        output: Fragment<Output<Context, Capture>>,
    ) -> Result<Fragment<Value<Context, Capture>>, Failure> {
        self.close(Inspection {
            input,
            output,
            context: self.context.clone(),
            origin: self.evidence(),
        })
    }

    pub fn close(
        &self,
        inspection: Inspection<Context, Capture>,
    ) -> Result<Fragment<Value<Context, Capture>>, Failure> {
        self.origin.history.permits(&inspection.origin.history)?;
        let input = self.accept(inspection.input)?;
        let output = self.accept(inspection.output)?;
        let mut evidence = input.evidence;
        evidence.append(output.evidence)?;
        evidence.append(inspection.origin)?;
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
            capture: std::marker::PhantomData,
            origin: Evidence {
                read: std::collections::BTreeMap::new(),
                context: BTreeSet::from([context]),
                history,
                proof: None,
                qualified: std::collections::BTreeMap::new(),
                capture: BTreeSet::new(),
            },
        }
    }

    pub fn inspect(&self, source: &Occurrence) -> Result<Fragment<Value>, Failure> {
        self.origin.history.permits(&source.history)?;
        let mut evidence = self.evidence();
        evidence.read.insert(source.identity, source.clone());
        source.value.context(&mut evidence.context);
        Ok(Fragment {
            value: source.value.clone(),
            evidence,
        })
    }
}

impl<Capture: Clone + Ord> Construction<context::Reference<Capture>, Capture> {
    pub fn local(&self) -> Self {
        Self {
            context: context::Reference::Local(0),
            origin: self.evidence(),
            capture: std::marker::PhantomData,
        }
    }

    pub fn capture<Item: crate::activation::Capture<Context = Capture>>(
        &self,
        value: Fragment<Item>,
    ) -> Result<Fragment<Item::Target>, Failure> {
        Ok(self.accept(value)?.map(Item::capture))
    }

    pub fn body(
        &self,
        rule: impl IntoIterator<Item = Fragment<Rule<context::Reference<Capture>, Capture>>>,
    ) -> Result<Fragment<Body<context::Reference<Capture>, Capture>>, Failure> {
        Ok(self
            .collect(rule)?
            .map(|rule| Body::nested(self.context.clone(), rule)))
    }
}

impl<Capture: Clone + Ord> Construction<Capture, Capture> {
    pub fn defer(&self) -> Construction<context::Reference<Capture>, Capture> {
        Construction {
            context: context::Reference::Captured(self.context.clone()),
            origin: self.evidence(),
            capture: std::marker::PhantomData,
        }
    }

    pub fn seal<Item: crate::activation::Seal<Context = Capture>>(
        &self,
        value: Fragment<Item>,
    ) -> Result<Fragment<Item::Target>, Failure> {
        let value = self.accept(value)?;
        Ok(Fragment {
            value: value.value.seal()?,
            evidence: value.evidence,
        })
    }

    pub fn body(
        &self,
        rule: impl IntoIterator<Item = Fragment<Rule<Capture, Capture>>>,
    ) -> Result<Fragment<Body<Capture, Capture>>, Failure> {
        Ok(self
            .collect(rule)?
            .map(|rule| Body::new(self.context.clone(), rule)))
    }
}
