use code::output::Output;
use code::particle::Particle;
use code::rule::Rule;
use code::scope::Scope;

pub(crate) struct Group {
    pub coherence: Vec<Particle>,
    pub rule: Vec<Rule>,
}

// Actions address the coherences a rule introduces: each plain output, and each scope's own
// coherences before those of the scopes it holds, so a scope with one coherence reads as one output.
pub(crate) fn list(rule: &Rule) -> Vec<&Particle> {
    let mut result = Vec::new();
    for output in rule.output() {
        match output {
            Output::Particle(particle) => result.push(particle),
            Output::Scope(scope) => collect(scope, &mut result),
        }
    }
    result
}

fn collect<'rule>(scope: &'rule Scope, result: &mut Vec<&'rule Particle>) {
    result.extend(scope.coherence());
    for nested in scope.scope() {
        collect(nested, result);
    }
}

pub(crate) fn map(rule: &Rule, change: impl FnMut(usize, &Particle, &mut Group)) -> Rule {
    let mut edit = Edit { counter: 0, change };
    let output = rule
        .output()
        .iter()
        .flat_map(|output| edit.output(output))
        .collect();
    Rule::new(rule.input().to_vec(), output)
}

struct Edit<Change> {
    counter: usize,
    change: Change,
}

impl<Change: FnMut(usize, &Particle, &mut Group)> Edit<Change> {
    fn output(&mut self, output: &Output) -> Vec<Output> {
        match output {
            Output::Particle(particle) => self.group(std::slice::from_ref(particle), &[], &[]),
            Output::Scope(scope) => self.group(scope.coherence(), scope.rule(), scope.scope()),
        }
    }

    fn group(&mut self, coherence: &[Particle], rule: &[Rule], scope: &[Scope]) -> Vec<Output> {
        let mut group = Group {
            coherence: Vec::new(),
            rule: rule.to_vec(),
        };
        for particle in coherence {
            (self.change)(self.counter, particle, &mut group);
            self.counter += 1;
        }
        let mut member = group
            .coherence
            .into_iter()
            .map(Output::Particle)
            .collect::<Vec<_>>();
        for nested in scope {
            member.extend(self.group(nested.coherence(), nested.rule(), nested.scope()));
        }
        Output::group(member, group.rule)
    }
}
