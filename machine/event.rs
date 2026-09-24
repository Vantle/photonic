use crate::coherence::{Coherence, Token};
use crate::flat::{Flat, run};
use crate::state::{Membership, State};
use code::atom::Atom;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    pub rule: usize,
    pub coherence: Vec<usize>,
    pub selection: Vec<Vec<u32>>,
}

struct Enumeration<'state, Visit> {
    program: &'state Flat,
    state: &'state State,
    membership: Vec<Membership>,
    visit: Visit,
}

pub fn enumerate(program: &Flat, state: &State, visit: impl FnMut(Event) -> bool) -> bool {
    let mut enumeration = Enumeration {
        program,
        state,
        membership: state.membership(),
        visit,
    };
    (0..program.rule().len()).all(|rule| enumeration.rule(rule))
}

impl<Visit: FnMut(Event) -> bool> Enumeration<'_, Visit> {
    fn rule(&mut self, rule: usize) -> bool {
        let candidate = self.program.rule()[rule]
            .input
            .iter()
            .map(|particle| {
                (0..self.state.coherence().len())
                    .filter(|&index| self.state.coherence()[index].contains(particle))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if candidate.iter().any(Vec::is_empty) {
            return true;
        }
        self.assign(rule, &candidate, &mut Vec::new())
    }

    fn assign(
        &mut self,
        rule: usize,
        candidate: &[Vec<usize>],
        assignment: &mut Vec<usize>,
    ) -> bool {
        let input = &self.program.rule()[rule].input;
        let position = assignment.len();
        if position == input.len() {
            return self.select(rule, assignment);
        }
        let floor = (position > 0 && input[position] == input[position - 1])
            .then(|| assignment[position - 1]);
        for &index in &candidate[position] {
            if floor.is_some_and(|floor| index <= floor) || assignment.contains(&index) {
                continue;
            }
            assignment.push(index);
            let proceed = self.assign(rule, candidate, assignment);
            assignment.pop();
            if !proceed {
                return false;
            }
        }
        true
    }

    fn select(&mut self, rule: usize, assignment: &[usize]) -> bool {
        let choice = self.program.rule()[rule]
            .input
            .iter()
            .zip(assignment)
            .map(|(particle, &index)| {
                choices(&self.state.coherence()[index], particle, &self.membership)
            })
            .collect::<Vec<_>>();
        let mut cursor = vec![0; choice.len()];
        loop {
            let event = Event {
                rule,
                coherence: assignment.to_vec(),
                selection: cursor
                    .iter()
                    .enumerate()
                    .map(|(position, &option)| choice[position][option].clone())
                    .collect(),
            };
            if !(self.visit)(event) {
                return false;
            }
            let mut position = 0;
            loop {
                if position == cursor.len() {
                    return true;
                }
                cursor[position] += 1;
                if cursor[position] < choice[position].len() {
                    break;
                }
                cursor[position] = 0;
                position += 1;
            }
        }
    }
}

fn shared(id: u32, membership: &[Membership]) -> bool {
    membership
        .binary_search_by_key(&id, |entry| entry.id)
        .is_ok()
}

fn group(token: &[Token], membership: &[Membership]) -> Vec<Vec<u32>> {
    let (shared, private): (Vec<&Token>, Vec<&Token>) =
        token.iter().partition(|token| shared(token.id, membership));
    std::iter::once(private.iter().map(|token| token.id).collect::<Vec<_>>())
        .filter(|private| !private.is_empty())
        .chain(shared.iter().map(|token| vec![token.id]))
        .collect()
}

fn combination(group: &[Vec<u32>], count: usize) -> Vec<Vec<u32>> {
    let Some((first, rest)) = group.split_first() else {
        return if count == 0 {
            vec![Vec::new()]
        } else {
            Vec::new()
        };
    };
    (0..=count.min(first.len()))
        .flat_map(|taken| {
            combination(rest, count - taken)
                .into_iter()
                .map(move |mut tail| {
                    tail.extend_from_slice(&first[..taken]);
                    tail
                })
        })
        .collect()
}

fn choices(coherence: &Coherence, particle: &[Atom], membership: &[Membership]) -> Vec<Vec<u32>> {
    let mut result = vec![Vec::new()];
    for (atom, count) in run(particle) {
        let option = combination(&group(coherence.range(atom), membership), count);
        result = result
            .iter()
            .flat_map(|prefix| {
                option.iter().map(move |option| {
                    let mut value = prefix.clone();
                    value.extend_from_slice(option);
                    value
                })
            })
            .collect();
    }
    for value in &mut result {
        value.sort_unstable();
    }
    result
}

pub fn apply(program: &Flat, state: &State, event: &[Event]) -> State {
    let mut removed = vec![false; state.coherence().len()];
    let mut next = state.next();
    let mut output = Vec::new();
    for event in event {
        let mut remainder: Vec<Token> = Vec::new();
        for (position, &index) in event.coherence.iter().enumerate() {
            removed[index] = true;
            let selection = &event.selection[position];
            remainder.extend(
                state.coherence()[index]
                    .token()
                    .iter()
                    .filter(|token| !selection.contains(&token.id)),
            );
        }
        remainder.sort_unstable_by_key(|token| token.id);
        remainder.dedup_by_key(|token| token.id);
        for particle in &program.rule()[event.rule].output {
            let mut token = remainder.clone();
            for &atom in particle {
                token.push(Token { atom, id: next });
                next += 1;
            }
            output.push(Coherence::new(token));
        }
    }
    State::assemble(
        state
            .coherence()
            .iter()
            .zip(&removed)
            .filter(|(_, removed)| !**removed)
            .map(|(coherence, _)| coherence.clone())
            .chain(output)
            .collect(),
        next,
    )
}
