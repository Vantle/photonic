use crate::program::{Instruction, Symbol};
use crate::state::State;

fn instruction<'value>(rule: &'value Instruction, pending: &mut Vec<&'value Symbol>) {
    pending.extend(rule.input.iter().flatten());
    pending.extend(rule.negative.iter().flatten().flatten());
    for output in &rule.output {
        pending.extend(&output.particle);
        if let Some(scope) = &output.body {
            for rule in &scope.rule {
                instruction(rule, pending);
            }
        }
    }
}

pub fn state(state: &State, limit: usize) -> usize {
    let mut pending = state
        .world
        .iter()
        .flat_map(|world| &world.particle)
        .chain(state.frame.iter().flat_map(|frame| &frame.held))
        .map(|token| &token.value)
        .collect::<Vec<_>>();
    for frame in &state.frame {
        for rule in &frame.scope.rule {
            instruction(rule, &mut pending);
        }
    }
    let mut size = 0;
    while let Some(value) = pending.pop() {
        if size == limit {
            return size.saturating_add(1);
        }
        size += 1;
        match value {
            Symbol::Atom(_) | Symbol::Variable(_) => {}
            Symbol::Structure(_, particle) => pending.extend(particle),
            Symbol::Rule(rule, _) => instruction(rule, &mut pending),
        }
    }
    size
}
