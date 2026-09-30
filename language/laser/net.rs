use super::ending;
use super::layout::Layout;
use super::makeup::Makeup;
use super::pool::Pool;
use super::scan;
use super::size::Size;
use super::taxonomy::{Draft, Taxonomy};
use super::trace::Trace;
use super::transition;
use crate::application::{Owner, Request};
use crate::catalog::Catalog;
use crate::flow::Binding;
use crate::program::{Program, Symbol};
use crate::render;
use crate::runtime::{Limit, Measure};
use crate::snapshot::{Definition, Node};
use crate::state::State;
use crate::status::Status;
use hashing::Builder;
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use thiserror::Error;

// A configuration whose root ties to one of its components is named whole, not by its parts, so
// it has no place in a net of parts.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum Unsupported {
    #[error("a configuration's root ties to one of its components")]
    Whole,
}

// Events of one part that change it the same way: the root after them, the kinds that replace the
// part's kinds, sorted, and how many distinct events do it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Entry {
    pub root: u32,
    pub produced: Vec<u32>,
    pub count: u32,
}

// A configuration an event joining several components leads to, with how many distinct events
// lead there that way.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Successor {
    pub marking: Makeup,
    pub count: u64,
}

// Whether an exploration also looks for a run that goes on forever, which keeps every edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cycle {
    Find,
    Ignore,
}

// Where exploring a net ends: whether it closed, how many configurations and events it found, the
// configurations where runs end, in the order they were found, whether a run can go on forever,
// when asked, and the work its grounding took.
pub struct Exploration {
    pub closed: bool,
    pub configuration: usize,
    pub event: u64,
    pub end: Vec<Makeup>,
    pub endless: Option<bool>,
    pub work: usize,
}

// A net names a configuration as Laser does, by its root and its components' kinds, and knows
// each plain event by the parts it touches. An event binds one component with the rules it sees,
// binds coherences of the root frame that lie in several components, or binds nothing but the
// root. So a part's events are found once, by the interpreter's matcher on the part alone, and
// each is applied once to the part; exploring then asks only the tables. The events of a
// component repeat for every copy of its kind, and the events joining several components repeat
// for every choice of copies. Grounding takes a step of work for each match it reads, and the
// tables it fills cost nothing to read again, so a net's work is the work its parts took.
pub struct Net {
    program: Arc<Program>,
    catalog: Arc<Catalog>,
    taxonomy: Taxonomy,
    start: Makeup,
    lone: HashMap<u32, Vec<Entry>, Builder>,
    single: HashMap<(u32, u32), Vec<Entry>, Builder>,
    join: IndexMap<Makeup, Vec<Entry>, Builder>,
    pattern: Vec<Vec<Vec<Symbol>>>,
    slot: Vec<(usize, usize)>,
    surface: HashMap<u32, Surface, Builder>,
    work: usize,
}

// What a rule joining several coherences can bind of a kind: how many coherences of the root frame
// a component of it holds, and the inputs, numbered across rules in order, that one could fill.
pub struct Surface {
    pub coherence: usize,
    pub reach: Vec<usize>,
}

fn group(effect: impl IntoIterator<Item = Makeup>) -> Vec<Entry> {
    let mut count = IndexMap::<Makeup, u32, Builder>::default();
    for makeup in effect {
        *count.entry(makeup).or_default() += 1;
    }
    count
        .into_iter()
        .map(|(makeup, count)| Entry {
            root: makeup.root,
            produced: makeup.kind,
            count,
        })
        .collect()
}

// The kinds of a makeup after its consumed kinds give way to the produced ones; both lists and the
// result are sorted.
fn replace(kind: &[u32], consumed: &[u32], produced: &[u32]) -> Vec<u32> {
    let mut result =
        Vec::with_capacity((kind.len() + produced.len()).saturating_sub(consumed.len()));
    let mut taken = consumed.iter().copied().peekable();
    let mut added = produced.iter().copied().peekable();
    for &value in kind {
        if taken.next_if_eq(&value).is_some() {
            continue;
        }
        while let Some(next) = added.next_if(|&next| next < value) {
            result.push(next);
        }
        result.push(value);
    }
    result.extend(added);
    result
}

fn choose(count: usize, taken: usize) -> u64 {
    (0..taken).fold(1, |product, index| {
        product * (count - index) as u64 / (index + 1) as u64
    })
}

// Each run of equal kinds in a sorted makeup, with its length.
fn run(kind: &[u32]) -> Vec<(u32, usize)> {
    let mut result = Vec::<(u32, usize)>::new();
    for &value in kind {
        match result.last_mut() {
            Some((last, count)) if *last == value => *count += 1,
            _ => result.push((value, 1)),
        }
    }
    result
}

// The makeup that names a configuration, its root and its components' kinds, sorted, numbering the
// roots and kinds the net meets for the first time; a configuration whose root ties to one of its
// components has none.
fn name(taxonomy: &mut Taxonomy, state: &State) -> Result<Makeup, Unsupported> {
    let draft = taxonomy.analyze(state);
    if matches!(draft, Draft::Whole(_)) {
        return Err(Unsupported::Whole);
    }
    let (root, mut kind) = taxonomy.intern(&draft);
    kind.sort_unstable();
    Ok(Makeup { root, kind })
}

// Whether some coherence of the root frame holds every value an input binds, so a component with
// these coherences could fill the input.
fn fills(coherence: &[Vec<Symbol>], input: &[Symbol]) -> bool {
    coherence
        .iter()
        .any(|world| input.iter().all(|symbol| world.contains(symbol)))
}

// Moves a counter whose digits each run over a range to its next value, the first digit fastest:
// the first digit below its range's last value goes up by one, and every digit before it returns
// to its range's start. False once every digit holds its range's last value.
fn step(digit: &mut [usize], range: &[Range<usize>]) -> bool {
    let Some(position) = digit
        .iter()
        .zip(range)
        .position(|(&value, range)| value + 1 < range.end)
    else {
        return false;
    };
    digit[position] += 1;
    for (value, range) in digit.iter_mut().zip(range).take(position) {
        *value = range.start;
    }
    true
}

impl Net {
    pub fn new(source: &frontend::source::Program) -> Result<Self, Unsupported> {
        let program = Arc::new(Program::new(source));
        let mut taxonomy = Taxonomy::default();
        let start = name(&mut taxonomy, &State::initial(&program))?;
        let pattern = program
            .rule
            .iter()
            .filter(|rule| rule.input.len() > 1)
            .map(|rule| rule.input.clone())
            .collect::<Vec<_>>();
        let slot = pattern
            .iter()
            .enumerate()
            .flat_map(|(rule, input)| (0..input.len()).map(move |index| (rule, index)))
            .collect();
        Ok(Self {
            catalog: Arc::new(Catalog::new(&program)),
            program,
            taxonomy,
            start,
            lone: HashMap::default(),
            single: HashMap::default(),
            join: IndexMap::default(),
            pattern,
            slot,
            surface: HashMap::default(),
            work: 0,
        })
    }

    pub fn start(&self) -> Makeup {
        self.start.clone()
    }

    // The marking of an exact target configuration, or none when a part of it is a kind the net
    // has never met, since then no marking it reached is the target.
    pub fn find(&self, target: &frontend::source::Program) -> Option<Makeup> {
        self.taxonomy.find(&State::target(&self.program, target))
    }

    // A marking as the interpreter reports a configuration, in its canonical form, with the
    // program's rules to name the rule values it holds.
    pub fn node(&self, marking: &Makeup) -> Node {
        render::Builder::new(&self.program).node(0, &self.state(marking), Status::Supported)
    }

    pub fn definition(&self) -> Vec<Definition> {
        render::Builder::new(&self.program).definition()
    }

    pub(crate) fn state(&self, marking: &Makeup) -> State {
        self.taxonomy.materialize(marking).canonical().state
    }

    // Every distinct plain event of the part holding the root and these kinds that involves every
    // component of it, as the plain engines identify and apply it, grouped by what it makes of the
    // part; an event involving fewer components is an event of a smaller part, grounded with it.
    // None when reading the part's matches would take the net's work past the allowance.
    fn ground(
        &mut self,
        root: u32,
        kind: &[u32],
        allowance: usize,
    ) -> Result<Option<Vec<Entry>>, Unsupported> {
        let makeup = Makeup {
            root,
            kind: kind.to_vec(),
        };
        let state = Arc::new(self.taxonomy.materialize(&makeup));
        let layout = Layout::new(&self.taxonomy, &makeup);
        let mut pool = Pool::default();
        let mut seen = IndexSet::<(usize, usize, usize, Binding), Builder>::default();
        for found in scan::scan(&self.catalog, &state) {
            if self.work >= allowance {
                return Ok(None);
            }
            self.work += 1;
            let Some(trace) = Trace::initial(&found, &state, None, &mut pool) else {
                continue;
            };
            let Some(binding) = trace.binding(&state, &pool) else {
                continue;
            };
            seen.insert((found.frame, found.owner, found.rule, binding));
        }
        let mut effect = Vec::with_capacity(seen.len());
        for (frame, owner, rule, binding) in seen {
            let involved = transition::involve(&layout, frame, owner, &binding);
            if !involved.iter().copied().eq(0..kind.len()) {
                continue;
            }
            let applied = crate::application::apply(Request {
                source: &state,
                scope: &self.program.scope,
                frame,
                owner: Owner::Frame(owner),
                rule: &self.program.rule[rule],
                binding: &binding,
            });
            effect.push(name(&mut self.taxonomy, &applied.state)?);
        }
        Ok(Some(group(effect)))
    }

    // The work grounding has taken so far.
    pub fn work(&self) -> usize {
        self.work
    }

    // The events that bind nothing but the root, once a visit grounded them.
    pub fn lone(&self, root: u32) -> Option<&[Entry]> {
        self.lone.get(&root).map(Vec::as_slice)
    }

    // The events of one component of a kind, with the root's rules, once a visit grounded them.
    pub fn single(&self, root: u32, kind: u32) -> Option<&[Entry]> {
        self.single.get(&(root, kind)).map(Vec::as_slice)
    }

    // A part of the root and two or more components that visits grounded, numbered in the order
    // they grounded them, with its events; none past the last.
    pub fn part(&self, id: usize) -> Option<(&Makeup, &[Entry])> {
        self.join
            .get_index(id)
            .map(|(makeup, entry)| (makeup, entry.as_slice()))
    }

    // Grounds every part a marking holds, and the multisets its kinds could join, as expanding the
    // marking does and in the same order, so a net that visits markings in the order this one
    // expands them numbers kinds as it does; then gives every configuration an event joining
    // several components leads to from the marking, in the order expanding it finds them. None
    // when grounding would take the net's work past the allowance, and then the marking is not
    // expanded.
    pub fn visit(
        &mut self,
        marking: &Makeup,
        allowance: usize,
    ) -> Result<Option<Vec<Successor>>, Unsupported> {
        let Some(part) = self.prepare(marking.root, &marking.kind, allowance)? else {
            return Ok(None);
        };
        Ok(Some(
            self.joined(&marking.kind, &part)
                .into_iter()
                .map(|(marking, count)| Successor { marking, count })
                .collect(),
        ))
    }

    // Grounds every part a makeup of the root and these kinds holds, and gives the parts of two or
    // more components its kinds could join, each with the number of ways to choose their copies;
    // none when grounding would take the net's work past the allowance.
    fn prepare(
        &mut self,
        root: u32,
        kind: &[u32],
        allowance: usize,
    ) -> Result<Option<Vec<(Makeup, u64)>>, Unsupported> {
        if !self.lone.contains_key(&root) {
            let Some(entry) = self.ground(root, &[], allowance)? else {
                return Ok(None);
            };
            self.lone.insert(root, entry);
        }
        let run = run(kind);
        for &(value, _) in &run {
            if !self.single.contains_key(&(root, value)) {
                let Some(entry) = self.ground(root, &[value], allowance)? else {
                    return Ok(None);
                };
                self.single.insert((root, value), entry);
            }
            self.survey(value);
        }
        let part = self.candidate(root, &run);
        for (makeup, _) in &part {
            if !self.join.contains_key(makeup) {
                let Some(entry) = self.ground(root, &makeup.kind, allowance)? else {
                    return Ok(None);
                };
                self.join.insert(makeup.clone(), entry);
            }
        }
        Ok(Some(part))
    }

    fn survey(&mut self, kind: u32) {
        if !self.surface.contains_key(&kind) {
            let surface = self.shape(kind);
            self.surface.insert(kind, surface);
        }
    }

    // Every configuration an event joining several components leads to from a makeup of these
    // kinds, through the grounded parts its kinds could join, with how many distinct events lead
    // there that way.
    fn joined(&self, kind: &[u32], part: &[(Makeup, u64)]) -> Vec<(Makeup, u64)> {
        let mut result = Vec::new();
        for (makeup, choice) in part {
            for entry in &self.join[makeup] {
                result.push((
                    Makeup {
                        root: entry.root,
                        kind: replace(kind, &makeup.kind, &entry.produced),
                    },
                    choice * u64::from(entry.count),
                ));
            }
        }
        result
    }

    // Whether the limits admit a marking's coherences, occurrences and scopes.
    pub fn admits(&self, marking: &Makeup, limit: Limit) -> bool {
        limit.admits(self.taxonomy.measure(marking))
    }

    pub fn size(&self, kind: u32) -> Size {
        self.taxonomy.size(kind)
    }

    // The occurrences a root's frame holds as values, which the occurrence limit weighs.
    pub fn occurrence(&self, root: u32) -> usize {
        match self.taxonomy.root(root) {
            super::taxonomy::Root::Hub(frame) => frame.held.len(),
            super::taxonomy::Root::Whole(state) => Measure::new(state).occurrence,
        }
    }

    // How many input coherences each rule joining several coherences has.
    pub fn arity(&self) -> Vec<usize> {
        self.pattern.iter().map(Vec::len).collect()
    }

    // What a rule joining several coherences can bind of a kind, which is all the values held by
    // each coherence of the root frame in it.
    pub fn shape(&self, kind: u32) -> Surface {
        let world = self
            .taxonomy
            .kind(kind)
            .world
            .iter()
            .filter(|world| world.frame == 0)
            .map(|world| world.particle.iter().map(|token| token.value).collect())
            .collect::<Vec<Vec<Symbol>>>();
        Surface {
            coherence: world.len(),
            reach: self
                .pattern
                .iter()
                .flatten()
                .enumerate()
                .filter(|(_, input)| fills(&world, input))
                .map(|(index, _)| index)
                .collect(),
        }
    }

    // The parts, the root and two or more components, that some rule joining several coherences
    // could bind in a makeup of the root and these kinds, each with the number of ways to choose
    // its copies, whichever present kind each input of the rule picks. Every part is kept once,
    // since its table holds the events of every rule that binds exactly those components. Each
    // present kind offers itself to the inputs it reaches, in the makeup's order, so a rule no
    // present kind reaches costs nothing.
    fn candidate(&self, root: u32, run: &[(u32, usize)]) -> Vec<(Makeup, u64)> {
        let mut offer = SmallVec::<[(usize, u32, usize); 16]>::new();
        for &(value, count) in run {
            offer.extend(
                self.surface[&value]
                    .reach
                    .iter()
                    .map(|&index| (index, value, count)),
            );
        }
        offer.sort_unstable_by_key(|&(index, value, _)| (index, value));
        let mut result = IndexMap::<Makeup, u64, Builder>::default();
        for group in offer.chunk_by(|left, right| self.slot[left.0].0 == self.slot[right.0].0) {
            let rule = self.slot[group[0].0].0;
            let option = group
                .chunk_by(|left, right| left.0 == right.0)
                .collect::<SmallVec<[_; 4]>>();
            if option.len() < self.pattern[rule].len() {
                continue;
            }
            let range = option
                .iter()
                .map(|input| 0..input.len())
                .collect::<SmallVec<[_; 4]>>();
            let mut index = SmallVec::<[usize; 4]>::from_elem(0, option.len());
            loop {
                let pick = index.iter().zip(&option).map(|(&position, input)| {
                    let (_, value, count) = input[position];
                    (value, count)
                });
                self.bind(root, pick, &mut result);
                if !step(&mut index, &range) {
                    break;
                }
            }
        }
        result.into_iter().collect()
    }

    // Keeps the parts a rule could bind once each of its inputs has picked a present kind. A rule
    // binds a distinct coherence for each input, and a copy of a kind hosts more than one input
    // only when it holds more than one coherence of the root frame, so a kind several inputs picked
    // gives a part a copy for each, or as few as one, and never more copies than the makeup has.
    fn bind(
        &self,
        root: u32,
        pick: impl IntoIterator<Item = (u32, usize)>,
        result: &mut IndexMap<Makeup, u64, Builder>,
    ) {
        let mut pick = pick.into_iter().collect::<SmallVec<[(u32, usize); 4]>>();
        pick.sort_unstable_by_key(|&(kind, _)| kind);
        let mut chosen = SmallVec::<[(u32, usize, usize); 4]>::new();
        for (kind, count) in pick {
            match chosen.last_mut() {
                Some((last, filled, _)) if *last == kind => *filled += 1,
                _ => chosen.push((kind, 1, count)),
            }
        }
        let range = chosen
            .iter()
            .map(|&(kind, filled, count)| {
                let low = if self.surface[&kind].coherence > 1 {
                    1
                } else {
                    filled
                };
                low..filled.min(count) + 1
            })
            .collect::<SmallVec<[_; 4]>>();
        if range.iter().any(Range::is_empty) {
            return;
        }
        let mut taken = range
            .iter()
            .map(|range| range.start)
            .collect::<SmallVec<[_; 4]>>();
        loop {
            let size = taken.iter().sum::<usize>();
            if size >= 2 {
                let mut kind = Vec::with_capacity(size);
                let mut choice = 1;
                for (&(value, _, count), &number) in chosen.iter().zip(&taken) {
                    kind.extend(std::iter::repeat_n(value, number));
                    choice *= choose(count, number);
                }
                result.insert(Makeup { root, kind }, choice);
            }
            if !step(&mut taken, &range) {
                return;
            }
        }
    }

    // Every configuration an event leads to from a makeup, with how many distinct events lead
    // there that way; none when grounding its parts would take the work past the allowance.
    fn expand(
        &mut self,
        makeup: &Makeup,
        allowance: usize,
    ) -> Result<Option<Vec<(Makeup, u64)>>, Unsupported> {
        let Some(part) = self.prepare(makeup.root, &makeup.kind, allowance)? else {
            return Ok(None);
        };
        let mut result = Vec::new();
        for entry in &self.lone[&makeup.root] {
            let kind = replace(&makeup.kind, &[], &entry.produced);
            result.push((
                Makeup {
                    root: entry.root,
                    kind,
                },
                u64::from(entry.count),
            ));
        }
        for (kind, count) in run(&makeup.kind) {
            for entry in &self.single[&(makeup.root, kind)] {
                let changed = replace(&makeup.kind, &[kind], &entry.produced);
                result.push((
                    Makeup {
                        root: entry.root,
                        kind: changed,
                    },
                    count as u64 * u64::from(entry.count),
                ));
            }
        }
        result.extend(self.joined(&makeup.kind, &part));
        Ok(Some(result))
    }

    // Explores every schedule of plain events breadth first, numbering configurations in the order
    // they are found, and stops short of any configuration or application the limits refuse, and
    // before the first configuration whose parts would take grounding past the budget. Looking for
    // a run that goes on forever keeps every edge, and searches them only when one leads to a
    // configuration found no later than its source, since every cycle has such an edge.
    pub fn explore(
        &mut self,
        budget: usize,
        limit: Limit,
        cycle: Cycle,
    ) -> Result<Exploration, Unsupported> {
        let start = self.work;
        let allowance = start.saturating_add(budget);
        let mut space = IndexSet::<Makeup, Builder>::default();
        space.insert(self.start.clone());
        let mut first = Vec::new();
        let mut edge = Vec::new();
        let mut backward = false;
        let mut event = 0;
        let mut end = Vec::new();
        let mut blocked = false;
        let mut spent = false;
        let mut next = 0;
        while next < space.len() {
            let Some(successor) = self.expand(&space[next], allowance)? else {
                spent = true;
                break;
            };
            if successor.is_empty() {
                end.push(next);
            }
            if cycle == Cycle::Find {
                first.push(edge.len());
            }
            for (makeup, count) in successor {
                if !limit.admits(self.taxonomy.measure(&makeup)) {
                    blocked = true;
                    continue;
                }
                let index = match space.get_index_of(&makeup) {
                    Some(index) => index,
                    None if space.len() >= limit.configuration => {
                        blocked = true;
                        continue;
                    }
                    None => space.insert_full(makeup).0,
                };
                event += count;
                if cycle == Cycle::Find {
                    backward |= index <= next;
                    edge.push(index);
                }
            }
            next += 1;
        }
        let endless = (cycle == Cycle::Find).then(|| {
            first.resize(space.len() + 1, edge.len());
            backward
                && ending::cyclic(space.len(), |node| {
                    edge[first[node]..first[node + 1]].iter().copied()
                })
        });
        Ok(Exploration {
            closed: !blocked && !spent,
            configuration: space.len(),
            event,
            end: end.into_iter().map(|index| space[index].clone()).collect(),
            endless,
            work: self.work - start,
        })
    }
}
