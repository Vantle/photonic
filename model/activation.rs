use crate::context::{Identity, Reference};
use crate::failure::Failure;
use crate::structure::{Body, Destination, Input, Output, Particle, Rule, Value};

fn value<Source: Ord + Clone, Target: Ord>(
    source: Value<Source>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(Rule<Reference>) -> Result<Rule<Reference>, Failure>,
) -> Result<Value<Target>, Failure> {
    match source {
        Value::Atom(atom) => Ok(Value::Atom(atom)),
        Value::Rule(rule) => Ok(Value::Rule(Box::new(map(*rule, context, declaration)?))),
    }
}

fn particle<Source: Ord + Clone, Target: Ord>(
    source: Particle<Source>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(Rule<Reference>) -> Result<Rule<Reference>, Failure>,
) -> Result<Particle<Target>, Failure> {
    Ok(Particle::new(
        source
            .value()
            .iter()
            .cloned()
            .map(|source| value(source, context, declaration))
            .collect::<Result<_, _>>()?,
    ))
}

fn map<Source: Ord + Clone, Target: Ord>(
    source: Rule<Source>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(Rule<Reference>) -> Result<Rule<Reference>, Failure>,
) -> Result<Rule<Target>, Failure> {
    let input = source
        .input
        .particle()
        .iter()
        .cloned()
        .map(|source| particle(source, context, declaration))
        .collect::<Result<_, _>>()?;
    let output = source
        .output
        .destination()
        .iter()
        .cloned()
        .map(|source| {
            let body = source
                .body
                .map(|source| {
                    let mut rule = source
                        .rule
                        .into_iter()
                        .map(declaration)
                        .collect::<Result<Vec<_>, _>>()?;
                    rule.sort();
                    Ok(Body {
                        context: context(source.context)?,
                        rule,
                    })
                })
                .transpose()?;
            Ok(Destination {
                particle: particle(source.particle, context, declaration)?,
                body,
            })
        })
        .collect::<Result<_, Failure>>()?;
    Ok(Rule {
        context: context(source.context)?,
        input: Input::new(input),
        output: Output::new(output),
    })
}

pub fn capture(source: Rule) -> Rule<Reference> {
    map(source, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
}

fn reference(source: Reference, depth: usize) -> Result<(), Failure> {
    if let Reference::Local(position) = source
        && position >= depth
    {
        return Err(Failure::Depth(position));
    }
    Ok(())
}

fn inspect(source: &Particle<Reference>, depth: usize) -> Result<(), Failure> {
    for value in source.value() {
        if let Value::Rule(rule) = value {
            validate(rule, depth)?;
        }
    }
    Ok(())
}

pub(crate) fn validate(source: &Rule<Reference>, depth: usize) -> Result<(), Failure> {
    reference(source.context, depth)?;
    for particle in source.input.particle() {
        inspect(particle, depth)?;
    }
    for destination in source.output.destination() {
        inspect(&destination.particle, depth)?;
        if let Some(body) = &destination.body {
            reference(body.context, depth)?;
            let depth = depth.checked_add(1).ok_or(Failure::Capacity)?;
            for rule in &body.rule {
                validate(rule, depth)?;
            }
        }
    }
    Ok(())
}

fn substitute(
    source: Rule<Reference>,
    owner: Identity,
    depth: usize,
) -> Result<Rule<Reference>, Failure> {
    map(
        source,
        &|context| match context {
            Reference::Local(position) if position == depth => Ok(Reference::Captured(owner)),
            Reference::Local(position) if position > depth => Err(Failure::Depth(position)),
            context => Ok(context),
        },
        &|rule| substitute(rule, owner, depth.checked_add(1).ok_or(Failure::Capacity)?),
    )
}

pub(crate) fn close(source: Rule<Reference>, owner: Identity) -> Result<Rule, Failure> {
    validate(&source, 1)?;
    map(
        source,
        &|context| match context {
            Reference::Captured(identity) => Ok(identity),
            Reference::Local(0) => Ok(owner),
            Reference::Local(position) => Err(Failure::Depth(position)),
        },
        &|rule| substitute(rule, owner, 1),
    )
}
