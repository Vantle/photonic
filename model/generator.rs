use crate::argument::Argument;
use crate::construction::Construction;
use crate::environment::Environment;
use crate::evidence::Evidence;
use crate::failure::Failure;
use crate::fragment::Fragment;
use crate::machine::Machine;
use crate::parameter::{Declaration, Parameter};
use crate::{structure, template};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Term {
    Value(template::Value),
    Quote(Box<Definition>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Definition {
    pub declaration: Declaration,
    pub term: Term,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Generator {
    definition: Definition,
    environment: Environment,
}

pub struct Invocation<'a> {
    term: &'a Term,
    environment: Environment,
    origin: Evidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Product {
    Value(Fragment<structure::Value>),
    Generator(Box<Fragment<Generator>>),
}

impl Generator {
    pub fn close(
        definition: Definition,
        environment: Environment,
        construction: &Construction,
    ) -> Result<Fragment<Self>, Failure> {
        let evidence = crate::quotation::check(&definition, &environment, construction)?;
        Ok(Fragment {
            value: Self {
                definition,
                environment,
            },
            evidence,
        })
    }
}

impl Fragment<Generator> {
    pub fn bind(
        &self,
        construction: &Construction,
        argument: Vec<Argument>,
    ) -> Result<Invocation<'_>, Failure> {
        let generator = &self.value;
        let declaration = &generator.definition.declaration;
        if declaration.parameter.len() != argument.len() {
            return Err(Failure::Arity {
                expected: declaration.parameter.len(),
                actual: argument.len(),
            });
        }
        construction.permits(&self.evidence)?;
        let mut origin = construction.evidence();
        origin.append(self.evidence.clone())?;
        let mut environment = generator.environment.nested(declaration.scope)?;
        for (parameter, argument) in declaration.parameter.iter().zip(argument) {
            match (parameter, argument) {
                (Parameter::Value(slot), Argument::Value(value)) => {
                    let value = construction.accept(value)?;
                    origin.append(value.evidence.clone())?;
                    environment.value = environment.value.bind(slot, value)?;
                }
                (Parameter::Particle(slot), Argument::Particle(value)) => {
                    let value = construction.accept(value)?;
                    origin.append(value.evidence.clone())?;
                    environment.particle = environment.particle.bind(slot, value)?;
                }
                (Parameter::Input(slot), Argument::Input(value)) => {
                    let value = construction.accept(value)?;
                    origin.append(value.evidence.clone())?;
                    environment.input = environment.input.bind(slot, value)?;
                }
                (Parameter::Output(slot), Argument::Output(value)) => {
                    let value = construction.accept(value)?;
                    origin.append(value.evidence.clone())?;
                    environment.output = environment.output.bind(slot, value)?;
                }
                (Parameter::Body(slot), Argument::Body(value)) => {
                    let value = construction.accept(value)?;
                    origin.append(value.evidence.clone())?;
                    environment.body = environment.body.bind(slot, value)?;
                }
                (parameter, argument) => {
                    return Err(Failure::Sort {
                        expected: parameter.identity().2,
                        actual: argument.sort(),
                    });
                }
            }
        }
        Ok(Invocation {
            term: &generator.definition.term,
            environment,
            origin,
        })
    }
}

impl Invocation<'_> {
    pub fn evaluate(&self, construction: &Construction) -> Result<Product, Failure> {
        construction.permits(&self.origin)?;
        match self.term {
            Term::Value(value) => {
                let mut result = value.instantiate(construction, &self.environment)?;
                result.evidence.append(self.origin.clone())?;
                Ok(Product::Value(result))
            }
            Term::Quote(definition) => {
                let mut result =
                    Generator::close(*definition.clone(), self.environment.clone(), construction)?;
                result.evidence.append(self.origin.clone())?;
                Ok(Product::Generator(Box::new(result)))
            }
        }
    }

    pub fn machine<'a>(&'a self, construction: &'a Construction) -> Result<Machine<'a>, Failure> {
        construction.permits(&self.origin)?;
        let Term::Value(value) = self.term else {
            return Err(Failure::Generator);
        };
        Ok(Machine::supported(
            value,
            construction,
            &self.environment,
            self.origin.clone(),
        ))
    }
}
