use crate::context::Reference;
use crate::failure::Failure;
use crate::structure::{Body, Destination, Input, Output, Particle, Rule, Value};

fn value<Source: Ord + Clone, Capture: Ord + Clone, Target: Ord, Closure: Ord>(
    source: Value<Source, Capture>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(
        Rule<Reference<Capture>, Capture>,
    ) -> Result<Rule<Reference<Closure>, Closure>, Failure>,
) -> Result<Value<Target, Closure>, Failure> {
    match source {
        Value::Atom(atom) => Ok(Value::Atom(atom)),
        Value::Rule(rule) => Ok(Value::Rule(Box::new(map(*rule, context, declaration)?))),
    }
}

fn particle<Source: Ord + Clone, Capture: Ord + Clone, Target: Ord, Closure: Ord>(
    source: Particle<Source, Capture>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(
        Rule<Reference<Capture>, Capture>,
    ) -> Result<Rule<Reference<Closure>, Closure>, Failure>,
) -> Result<Particle<Target, Closure>, Failure> {
    Ok(Particle::new(
        source
            .value()
            .iter()
            .cloned()
            .map(|source| value(source, context, declaration))
            .collect::<Result<_, _>>()?,
    ))
}

fn input<Source: Ord + Clone, Capture: Ord + Clone, Target: Ord, Closure: Ord>(
    source: Input<Source, Capture>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(
        Rule<Reference<Capture>, Capture>,
    ) -> Result<Rule<Reference<Closure>, Closure>, Failure>,
) -> Result<Input<Target, Closure>, Failure> {
    Ok(Input::new(
        source
            .particle()
            .iter()
            .cloned()
            .map(|source| particle(source, context, declaration))
            .collect::<Result<_, _>>()?,
    ))
}

fn body<Source, Capture: Ord + Clone, Target, Closure: Ord>(
    source: Body<Source, Capture>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(
        Rule<Reference<Capture>, Capture>,
    ) -> Result<Rule<Reference<Closure>, Closure>, Failure>,
) -> Result<Body<Target, Closure>, Failure> {
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

fn destination<Source: Ord + Clone, Capture: Ord + Clone, Target: Ord, Closure: Ord>(
    source: Destination<Source, Capture>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(
        Rule<Reference<Capture>, Capture>,
    ) -> Result<Rule<Reference<Closure>, Closure>, Failure>,
) -> Result<Destination<Target, Closure>, Failure> {
    Ok(Destination {
        particle: particle(source.particle, context, declaration)?,
        body: source
            .body
            .map(|source| body(source, context, declaration))
            .transpose()?,
    })
}

fn output<Source: Ord + Clone, Capture: Ord + Clone, Target: Ord, Closure: Ord>(
    source: Output<Source, Capture>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(
        Rule<Reference<Capture>, Capture>,
    ) -> Result<Rule<Reference<Closure>, Closure>, Failure>,
) -> Result<Output<Target, Closure>, Failure> {
    Ok(Output::new(
        source
            .destination()
            .iter()
            .cloned()
            .map(|source| destination(source, context, declaration))
            .collect::<Result<_, _>>()?,
    ))
}

fn map<Source: Ord + Clone, Capture: Ord + Clone, Target: Ord, Closure: Ord>(
    source: Rule<Source, Capture>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
    declaration: &impl Fn(
        Rule<Reference<Capture>, Capture>,
    ) -> Result<Rule<Reference<Closure>, Closure>, Failure>,
) -> Result<Rule<Target, Closure>, Failure> {
    Ok(Rule {
        context: context(source.context)?,
        input: input(source.input, context, declaration)?,
        output: output(source.output, context, declaration)?,
    })
}

pub fn capture<Context: Clone + Ord>(
    source: Rule<Context, Context>,
) -> Rule<Reference<Context>, Context> {
    map(source, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
}

fn relocate<Source: Clone + Ord, Target: Ord>(
    source: Rule<Reference<Source>, Source>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
) -> Result<Rule<Reference<Target>, Target>, Failure> {
    map(
        source,
        &|reference| match reference {
            Reference::Captured(identity) => Ok(Reference::Captured(context(identity)?)),
            Reference::Local(position) => Ok(Reference::Local(position)),
        },
        &|source| relocate(source, context),
    )
}

pub(crate) fn rename<Source: Clone + Ord, Target: Ord>(
    source: Value<Source, Source>,
    context: &impl Fn(Source) -> Result<Target, Failure>,
) -> Result<Value<Target, Target>, Failure> {
    value(source, context, &|source| relocate(source, context))
}

fn reference<Context>(source: &Reference<Context>, depth: usize) -> Result<(), Failure> {
    if let Reference::Local(position) = source
        && *position >= depth
    {
        return Err(Failure::Depth(*position));
    }
    Ok(())
}

fn inspect<Context: Clone + Ord>(
    source: &Particle<Reference<Context>, Context>,
    depth: usize,
) -> Result<(), Failure> {
    for value in source.value() {
        if let Value::Rule(rule) = value {
            validate(rule, depth)?;
        }
    }
    Ok(())
}

pub(crate) fn validate<Context: Clone + Ord>(
    source: &Rule<Reference<Context>, Context>,
    depth: usize,
) -> Result<(), Failure> {
    reference(&source.context, depth)?;
    for particle in source.input.particle() {
        inspect(particle, depth)?;
    }
    for destination in source.output.destination() {
        inspect(&destination.particle, depth)?;
        if let Some(body) = &destination.body {
            reference(&body.context, depth)?;
            let depth = depth.checked_add(1).ok_or(Failure::Capacity)?;
            for rule in &body.rule {
                validate(rule, depth)?;
            }
        }
    }
    Ok(())
}

fn substitute<Context: Clone + Ord>(
    source: Rule<Reference<Context>, Context>,
    owner: Context,
    depth: usize,
) -> Result<Rule<Reference<Context>, Context>, Failure> {
    map(
        source,
        &|context| match context {
            Reference::Local(position) if position == depth => {
                Ok(Reference::Captured(owner.clone()))
            }
            Reference::Local(position) if position > depth => Err(Failure::Depth(position)),
            context => Ok(context),
        },
        &|rule| {
            substitute(
                rule,
                owner.clone(),
                depth.checked_add(1).ok_or(Failure::Capacity)?,
            )
        },
    )
}

pub(crate) fn close<Context: Clone + Ord>(
    source: Rule<Reference<Context>, Context>,
    owner: Context,
) -> Result<Rule<Context, Context>, Failure> {
    validate(&source, 1)?;
    map(
        source,
        &|context| match context {
            Reference::Captured(identity) => Ok(identity),
            Reference::Local(0) => Ok(owner.clone()),
            Reference::Local(position) => Err(Failure::Depth(position)),
        },
        &|rule| substitute(rule, owner.clone(), 1),
    )
}

pub trait Capture {
    type Context;
    type Target;
    fn capture(self) -> Self::Target;
}

pub trait Seal {
    type Context;
    type Target;
    fn seal(self) -> Result<Self::Target, Failure>;
}

fn identity<Context>(context: Reference<Context>) -> Result<Context, Failure> {
    match context {
        Reference::Captured(identity) => Ok(identity),
        Reference::Local(depth) => Err(Failure::Depth(depth)),
    }
}

fn declaration<Context: Clone + Ord>(
    rule: Rule<Reference<Context>, Context>,
) -> Result<Rule<Reference<Context>, Context>, Failure> {
    validate(&rule, 1)?;
    Ok(rule)
}

impl<Context: Clone + Ord> Capture for Value<Context, Context> {
    type Context = Context;
    type Target = Value<Reference<Context>, Context>;
    fn capture(self) -> Self::Target {
        value(self, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
    }
}

impl<Context: Clone + Ord> Seal for Value<Reference<Context>, Context> {
    type Context = Context;
    type Target = Value<Context, Context>;
    fn seal(self) -> Result<Self::Target, Failure> {
        value(self, &identity, &declaration)
    }
}

impl<Context: Clone + Ord> Capture for Particle<Context, Context> {
    type Context = Context;
    type Target = Particle<Reference<Context>, Context>;
    fn capture(self) -> Self::Target {
        particle(self, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
    }
}

impl<Context: Clone + Ord> Seal for Particle<Reference<Context>, Context> {
    type Context = Context;
    type Target = Particle<Context, Context>;
    fn seal(self) -> Result<Self::Target, Failure> {
        particle(self, &identity, &declaration)
    }
}

impl<Context: Clone + Ord> Capture for Input<Context, Context> {
    type Context = Context;
    type Target = Input<Reference<Context>, Context>;
    fn capture(self) -> Self::Target {
        input(self, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
    }
}

impl<Context: Clone + Ord> Seal for Input<Reference<Context>, Context> {
    type Context = Context;
    type Target = Input<Context, Context>;
    fn seal(self) -> Result<Self::Target, Failure> {
        input(self, &identity, &declaration)
    }
}

impl<Context: Clone + Ord> Capture for Destination<Context, Context> {
    type Context = Context;
    type Target = Destination<Reference<Context>, Context>;
    fn capture(self) -> Self::Target {
        destination(self, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
    }
}

impl<Context: Clone + Ord> Seal for Destination<Reference<Context>, Context> {
    type Context = Context;
    type Target = Destination<Context, Context>;
    fn seal(self) -> Result<Self::Target, Failure> {
        destination(self, &identity, &declaration)
    }
}

impl<Context: Clone + Ord> Capture for Output<Context, Context> {
    type Context = Context;
    type Target = Output<Reference<Context>, Context>;
    fn capture(self) -> Self::Target {
        output(self, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
    }
}

impl<Context: Clone + Ord> Seal for Output<Reference<Context>, Context> {
    type Context = Context;
    type Target = Output<Context, Context>;
    fn seal(self) -> Result<Self::Target, Failure> {
        output(self, &identity, &declaration)
    }
}

impl<Context: Clone + Ord> Capture for Body<Context, Context> {
    type Context = Context;
    type Target = Body<Reference<Context>, Context>;
    fn capture(self) -> Self::Target {
        body(self, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
    }
}

impl<Context: Clone + Ord> Seal for Body<Reference<Context>, Context> {
    type Context = Context;
    type Target = Body<Context, Context>;
    fn seal(self) -> Result<Self::Target, Failure> {
        body(self, &identity, &declaration)
    }
}

impl<Context: Clone + Ord> Capture for Rule<Context, Context> {
    type Context = Context;
    type Target = Rule<Reference<Context>, Context>;
    fn capture(self) -> Self::Target {
        map(self, &|context| Ok(Reference::Captured(context)), &Ok).unwrap()
    }
}

impl<Context: Clone + Ord> Seal for Rule<Reference<Context>, Context> {
    type Context = Context;
    type Target = Rule<Context, Context>;
    fn seal(self) -> Result<Self::Target, Failure> {
        map(self, &identity, &declaration)
    }
}
