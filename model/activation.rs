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

fn input<Source: Ord + Clone, Target: Ord>(
    source: Input<Source>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(Rule<Reference>) -> Result<Rule<Reference>, Failure>,
) -> Result<Input<Target>, Failure> {
    Ok(Input::new(
        source
            .particle()
            .iter()
            .cloned()
            .map(|source| particle(source, context, declaration))
            .collect::<Result<_, _>>()?,
    ))
}

fn body<Source, Target>(
    source: Body<Source>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(Rule<Reference>) -> Result<Rule<Reference>, Failure>,
) -> Result<Body<Target>, Failure> {
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
}

fn destination<Source: Ord + Clone, Target: Ord>(
    source: Destination<Source>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(Rule<Reference>) -> Result<Rule<Reference>, Failure>,
) -> Result<Destination<Target>, Failure> {
    Ok(Destination {
        particle: particle(source.particle, context, declaration)?,
        body: source
            .body
            .map(|source| body(source, context, declaration))
            .transpose()?,
    })
}

fn output<Source: Ord + Clone, Target: Ord>(
    source: Output<Source>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(Rule<Reference>) -> Result<Rule<Reference>, Failure>,
) -> Result<Output<Target>, Failure> {
    Ok(Output::new(
        source
            .destination()
            .iter()
            .cloned()
            .map(|source| destination(source, context, declaration))
            .collect::<Result<_, _>>()?,
    ))
}

fn map<Source: Ord + Clone, Target: Ord>(
    source: Rule<Source>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(Rule<Reference>) -> Result<Rule<Reference>, Failure>,
) -> Result<Rule<Target>, Failure> {
    Ok(Rule {
        context: context(source.context)?,
        input: input(source.input, context, declaration)?,
        output: output(source.output, context, declaration)?,
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

pub trait Capture {
    type Target;
    fn capture(self) -> Self::Target;
}

pub trait Seal {
    type Target;
    fn seal(self) -> Result<Self::Target, Failure>;
}

fn identity(context: Reference) -> Result<Identity, Failure> {
    match context {
        Reference::Captured(identity) => Ok(identity),
        Reference::Local(depth) => Err(Failure::Depth(depth)),
    }
}

fn declaration(rule: Rule<Reference>) -> Result<Rule<Reference>, Failure> {
    validate(&rule, 1)?;
    Ok(rule)
}

impl Capture for Value {
    type Target = Value<Reference>;
    fn capture(self) -> Self::Target {
        value(self, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
    }
}

impl Seal for Value<Reference> {
    type Target = Value;
    fn seal(self) -> Result<Self::Target, Failure> {
        value(self, &identity, &declaration)
    }
}

impl Capture for Particle {
    type Target = Particle<Reference>;
    fn capture(self) -> Self::Target {
        particle(self, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
    }
}

impl Seal for Particle<Reference> {
    type Target = Particle;
    fn seal(self) -> Result<Self::Target, Failure> {
        particle(self, &identity, &declaration)
    }
}

impl Capture for Input {
    type Target = Input<Reference>;
    fn capture(self) -> Self::Target {
        input(self, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
    }
}

impl Seal for Input<Reference> {
    type Target = Input;
    fn seal(self) -> Result<Self::Target, Failure> {
        input(self, &identity, &declaration)
    }
}

impl Capture for Destination {
    type Target = Destination<Reference>;
    fn capture(self) -> Self::Target {
        destination(self, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
    }
}

impl Seal for Destination<Reference> {
    type Target = Destination;
    fn seal(self) -> Result<Self::Target, Failure> {
        destination(self, &identity, &declaration)
    }
}

impl Capture for Output {
    type Target = Output<Reference>;
    fn capture(self) -> Self::Target {
        output(self, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
    }
}

impl Seal for Output<Reference> {
    type Target = Output;
    fn seal(self) -> Result<Self::Target, Failure> {
        output(self, &identity, &declaration)
    }
}

impl Capture for Body {
    type Target = Body<Reference>;
    fn capture(self) -> Self::Target {
        body(self, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
    }
}

impl Seal for Body<Reference> {
    type Target = Body;
    fn seal(self) -> Result<Self::Target, Failure> {
        body(self, &identity, &declaration)
    }
}

impl Capture for Rule {
    type Target = Rule<Reference>;
    fn capture(self) -> Self::Target {
        map(self, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
    }
}

impl Seal for Rule<Reference> {
    type Target = Rule;
    fn seal(self) -> Result<Self::Target, Failure> {
        map(self, &identity, &declaration)
    }
}
