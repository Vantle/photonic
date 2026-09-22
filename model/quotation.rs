use crate::binding::Binding;
use crate::construction::Construction;
use crate::environment::Environment;
use crate::evidence::Evidence;
use crate::failure::Failure;
use crate::generator::{Definition, Term};
use crate::parameter::{Declaration, Sort};
use crate::slot::Slot;
use crate::template;

struct Quotation<'a, Capture> {
    declaration: Vec<&'a Declaration<Capture>>,
    environment: &'a Environment<Capture>,
    construction: &'a Construction<Capture, Capture>,
    evidence: Evidence,
}

pub(crate) fn check<Capture: Clone + Ord>(
    definition: &Definition<Capture>,
    environment: &Environment<Capture>,
    construction: &Construction<Capture, Capture>,
) -> Result<Evidence, Failure> {
    let mut quotation = Quotation {
        declaration: Vec::new(),
        environment,
        construction,
        evidence: construction.evidence(),
    };
    quotation.definition(definition)?;
    Ok(quotation.evidence)
}

impl<'a, Capture: Clone + Ord> Quotation<'a, Capture> {
    fn reference<Value: Clone>(
        &mut self,
        slot: &Slot<Value>,
        sort: Sort,
        binding: &Binding<Value>,
    ) -> Result<(), Failure> {
        for declaration in self.declaration.iter().rev() {
            if declaration.contains(slot, sort)? {
                return Ok(());
            }
        }
        let fragment = self.construction.accept(binding.resolve(slot)?)?;
        self.evidence.append(fragment.evidence)?;
        Ok(())
    }

    fn definition(&mut self, definition: &'a Definition<Capture>) -> Result<(), Failure> {
        if self
            .declaration
            .iter()
            .any(|value| value.scope == definition.declaration.scope)
        {
            return Err(Failure::Scope(definition.declaration.scope));
        }
        self.environment.nested(definition.declaration.scope)?;
        self.declaration.push(&definition.declaration);
        match &definition.term {
            Term::Value(value) => self.value(value)?,
            Term::Quote(definition) => self.definition(definition)?,
        }
        self.declaration.pop();
        Ok(())
    }

    fn value(&mut self, value: &template::Value<Capture>) -> Result<(), Failure> {
        match value {
            template::Value::Atom(_) => Ok(()),
            template::Value::Reference(slot) => {
                self.reference(slot, Sort::Value, &self.environment.value)
            }
            template::Value::Rule(rule) => self.rule(rule),
        }
    }

    fn particle(&mut self, particle: &template::Particle<Capture>) -> Result<(), Failure> {
        match particle {
            template::Particle::Reference(slot) => {
                self.reference(slot, Sort::Particle, &self.environment.particle)
            }
            template::Particle::Build(value) => {
                for value in value {
                    self.value(value)?;
                }
                Ok(())
            }
        }
    }

    fn input(&mut self, input: &template::Input<Capture>) -> Result<(), Failure> {
        match input {
            template::Input::Reference(slot) => {
                self.reference(slot, Sort::Input, &self.environment.input)
            }
            template::Input::Build(particle) => {
                for particle in particle {
                    self.particle(particle)?;
                }
                Ok(())
            }
        }
    }

    fn output(&mut self, output: &template::Output<Capture>) -> Result<(), Failure> {
        match output {
            template::Output::Reference(slot) => {
                self.reference(slot, Sort::Output, &self.environment.output)
            }
            template::Output::Build(destination) => {
                for destination in destination {
                    self.particle(&destination.particle)?;
                    if let Some(body) = &destination.body {
                        self.body(body)?;
                    }
                }
                Ok(())
            }
        }
    }

    fn body(&mut self, body: &template::Body<Capture>) -> Result<(), Failure> {
        match body {
            template::Body::Reference(slot) => {
                self.reference(slot, Sort::Body, &self.environment.body)
            }
            template::Body::Build(rule) => {
                for rule in rule {
                    self.rule(rule)?;
                }
                Ok(())
            }
        }
    }

    fn rule(&mut self, rule: &template::Rule<Capture>) -> Result<(), Failure> {
        self.input(&rule.input)?;
        self.output(&rule.output)
    }
}
