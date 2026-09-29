use super::layout::Layout;
use super::pool::Pool;
use super::scan;
use super::taxonomy::{Draft, Makeup, Taxonomy};
use super::trace::Trace;
use super::transition;
use super::{Disagreement, Laser};
use crate::application::{Owner, Request};
use crate::catalog::Catalog;
use crate::flow::Binding;
use crate::program::{Program, Symbol};
use crate::render;
use crate::runtime::Limit;
use crate::snapshot::{Definition, Node};
use crate::state::State;
use crate::status::Status;
use hashing::Builder;
use indexmap::{IndexMap, IndexSet};
use std::collections::HashMap;
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

// A configuration of the net: its root and its components' kinds, sorted.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Marking {
    pub root: u32,
    pub kind: Vec<u32>,
}

// A configuration an event joining several components leads to, with how many distinct events
// lead there that way.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Successor {
    pub marking: Marking,
    pub count: u64,
}

// What a component of a kind holds beside the root: its coherences, occurrences and frames.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Size {
    pub world: usize,
    pub occurrence: usize,
    pub frame: usize,
}

// Whether an exploration also looks for a run that goes on forever, which keeps every edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cycle {
    Find,
    Ignore,
}

// Where exploring a net ends: whether it closed, how many configurations and events it found, the
// configurations where runs end, in the order they were found, and, when asked, whether a run can
// go on forever.
pub struct Exploration {
    pub closed: bool,
    pub configuration: usize,
    pub event: u64,
    pub end: Vec<Marking>,
    pub endless: Option<bool>,
}

// A net names a configuration as Laser does, by its root and its components' kinds, and knows
// each plain event by the parts it touches. An event binds one component with the rules it sees,
// binds coherences of the root frame that lie in several components, or binds nothing but the
// root. So a part's events are found once, by the interpreter's matcher on the part alone, and
// each is applied once to the part; exploring then asks only the tables. The events of a
// component repeat for every copy of its kind, and the events joining several components repeat
// for every choice of copies.
pub struct Net {
    program: Arc<Program>,
    catalog: Arc<Catalog>,
    taxonomy: Taxonomy,
    start: Makeup,
    lone: HashMap<u32, Vec<Entry>, Builder>,
    single: HashMap<(u32, u32), Vec<Entry>, Builder>,
    join: HashMap<(u32, Vec<u32>), Vec<Entry>, Builder>,
    pattern: Vec<Vec<Vec<Symbol>>>,
    surface: HashMap<u32, Vec<Vec<Symbol>>, Builder>,
}

// The parts an identity touches and what applying it to them makes.
struct Effect {
    involved: Vec<usize>,
    root: u32,
    produced: Vec<u32>,
}

fn group(effect: impl IntoIterator<Item = (u32, Vec<u32>)>) -> Vec<Entry> {
    let mut count = IndexMap::<(u32, Vec<u32>), u32, Builder>::default();
    for key in effect {
        *count.entry(key).or_default() += 1;
    }
    count
        .into_iter()
        .map(|((root, produced), count)| Entry {
            root,
            produced,
            count,
        })
        .collect()
}

// The kinds of a makeup after its consumed kinds give way to the produced ones; both lists and the
// result are sorted.
fn replace(kind: &[u32], consumed: &[u32], produced: &[u32]) -> Vec<u32> {
    let mut kept = Vec::with_capacity(kind.len());
    let mut taken = consumed.iter().peekable();
    for &value in kind {
        if taken.peek() == Some(&&value) {
            taken.next();
            continue;
        }
        kept.push(value);
    }
    let mut result = Vec::with_capacity(kept.len() + produced.len());
    let (mut left, mut right) = (kept.iter().peekable(), produced.iter().peekable());
    loop {
        match (left.peek(), right.peek()) {
            (Some(&&first), Some(&&second)) if first <= second => {
                result.push(first);
                left.next();
            }
            (_, Some(&&second)) => {
                result.push(second);
                right.next();
            }
            (Some(&&first), None) => {
                result.push(first);
                left.next();
            }
            (None, None) => break,
        }
    }
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

impl Net {
    pub fn new(source: &frontend::source::Program) -> Result<Self, Unsupported> {
        let program = Arc::new(Program::new(source));
        let mut taxonomy = Taxonomy::default();
        let initial = State::initial(&program);
        let draft = taxonomy.analyze(&initial);
        if matches!(draft, Draft::Whole(_)) {
            return Err(Unsupported::Whole);
        }
        let (root, mut kind) = taxonomy.intern(&draft);
        kind.sort_unstable();
        let pattern = program
            .rule
            .iter()
            .filter(|rule| rule.input.len() > 1)
            .map(|rule| rule.input.clone())
            .collect();
        Ok(Self {
            catalog: Arc::new(Catalog::new(&program)),
            program,
            taxonomy,
            start: Makeup { root, kind },
            lone: HashMap::default(),
            single: HashMap::default(),
            join: HashMap::default(),
            pattern,
            surface: HashMap::default(),
        })
    }

    pub fn start(&self) -> Marking {
        Marking {
            root: self.start.root,
            kind: self.start.kind.clone(),
        }
    }

    // The marking of an exact target configuration, or none when a part of it is a kind the net
    // has never met, since then no marking it reached is the target.
    pub fn find(&self, target: &frontend::source::Program) -> Option<Marking> {
        let makeup = self.taxonomy.find(&State::target(&self.program, target))?;
        Some(Marking {
            root: makeup.root,
            kind: makeup.kind,
        })
    }

    // A marking as the interpreter reports a configuration, in its canonical form, with the
    // program's rules to name the rule values it holds.
    pub fn node(&self, marking: &Marking) -> Node {
        render::Builder::new(&self.program).node(0, &self.state(marking), Status::Supported)
    }

    pub fn definition(&self) -> Vec<Definition> {
        render::Builder::new(&self.program).definition()
    }

    pub(crate) fn state(&self, marking: &Marking) -> State {
        self.taxonomy
            .materialize(&Makeup {
                root: marking.root,
                kind: marking.kind.clone(),
            })
            .canonical()
            .state
    }

    // Every distinct plain event of the part holding the root and these kinds, with the parts it
    // touches and what it makes of them, as the plain engines identify and apply it.
    fn part(&mut self, root: u32, kind: &[u32]) -> Result<Vec<Effect>, Unsupported> {
        let makeup = Makeup {
            root,
            kind: kind.to_vec(),
        };
        let state = Arc::new(self.taxonomy.materialize(&makeup));
        let layout = Layout::new(&self.taxonomy, &makeup);
        let mut pool = Pool::default();
        let mut seen = IndexSet::<(usize, usize, usize, Binding), Builder>::default();
        for found in scan::scan(&self.catalog, &state) {
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
            let involved = transition::involve(&layout, frame, owner, &binding).to_vec();
            let applied = crate::application::apply(Request {
                source: &state,
                scope: &self.program.scope,
                frame,
                owner: Owner::Frame(owner),
                rule: &self.program.rule[rule],
                binding: &binding,
            });
            let draft = self.taxonomy.analyze(&applied.state);
            if matches!(draft, Draft::Whole(_)) {
                return Err(Unsupported::Whole);
            }
            let (root, mut produced) = self.taxonomy.intern(&draft);
            produced.sort_unstable();
            effect.push(Effect {
                involved,
                root,
                produced,
            });
        }
        Ok(effect)
    }

    fn ground(&mut self, root: u32, kind: &[u32]) -> Result<Vec<Entry>, Unsupported> {
        let every = (0..kind.len()).collect::<Vec<_>>();
        let effect = self.part(root, kind)?;
        Ok(group(
            effect
                .into_iter()
                .filter(|effect| effect.involved == every)
                .map(|effect| (effect.root, effect.produced)),
        ))
    }

    // The events that bind nothing but the root.
    pub fn lone(&mut self, root: u32) -> Result<&[Entry], Unsupported> {
        if !self.lone.contains_key(&root) {
            let entry = self.ground(root, &[])?;
            self.lone.insert(root, entry);
        }
        Ok(&self.lone[&root])
    }

    // The events of one component of a kind, with the root's rules.
    pub fn single(&mut self, root: u32, kind: u32) -> Result<&[Entry], Unsupported> {
        if !self.single.contains_key(&(root, kind)) {
            let entry = self.ground(root, &[kind])?;
            self.single.insert((root, kind), entry);
        }
        Ok(&self.single[&(root, kind)])
    }

    // Grounds every part a marking holds, and the multisets its kinds could join, as expanding the
    // marking does and in the same order, so a net that visits markings in the order this one
    // expands them numbers kinds as it does.
    pub fn visit(&mut self, marking: &Marking) -> Result<(), Unsupported> {
        self.prepare(&Makeup {
            root: marking.root,
            kind: marking.kind.clone(),
        })
    }

    fn prepare(&mut self, makeup: &Makeup) -> Result<(), Unsupported> {
        self.lone(makeup.root)?;
        for (kind, _) in run(&makeup.kind) {
            self.single(makeup.root, kind)?;
            self.survey(kind);
        }
        for (multiset, _) in self.candidate(makeup) {
            let key = (makeup.root, multiset);
            if !self.join.contains_key(&key) {
                let entry = self.ground(key.0, &key.1)?;
                self.join.insert(key, entry);
            }
        }
        Ok(())
    }

    fn survey(&mut self, kind: u32) {
        if !self.surface.contains_key(&kind) {
            let surface = self.shape(kind);
            self.surface.insert(kind, surface);
        }
    }

    // Every configuration an event joining several components leads to from a marking, in the
    // order expanding the marking finds them.
    pub fn joined(&mut self, marking: &Marking) -> Result<Vec<Successor>, Unsupported> {
        let makeup = Makeup {
            root: marking.root,
            kind: marking.kind.clone(),
        };
        self.prepare(&makeup)?;
        let mut result = Vec::new();
        for (multiset, ways) in self.candidate(&makeup) {
            for entry in &self.join[&(makeup.root, multiset.clone())] {
                result.push(Successor {
                    marking: Marking {
                        root: entry.root,
                        kind: replace(&makeup.kind, &multiset, &entry.produced),
                    },
                    count: ways * u64::from(entry.count),
                });
            }
        }
        Ok(result)
    }

    pub fn size(&self, kind: u32) -> Size {
        let size = self.taxonomy.kind(kind).1;
        Size {
            world: size.world,
            occurrence: size.occurrence,
            frame: size.frame,
        }
    }

    // The occurrences a root's frame holds.
    pub fn occurrence(&self, root: u32) -> usize {
        match self.taxonomy.root(root) {
            super::taxonomy::Root::Hub(frame) => frame.size(),
            super::taxonomy::Root::Whole(state) => state.size(),
        }
    }

    // How many input coherences each rule joining several coherences has.
    pub fn join(&self) -> Vec<usize> {
        self.pattern.iter().map(Vec::len).collect()
    }

    // The inputs of the joining rules, numbered across rules in order, that a coherence of the root
    // frame in a component of this kind could fill.
    pub fn reach(&mut self, kind: u32) -> Vec<usize> {
        self.survey(kind);
        let surface = &self.surface[&kind];
        self.pattern
            .iter()
            .flatten()
            .enumerate()
            .filter(|(_, input)| {
                surface
                    .iter()
                    .any(|world| input.iter().all(|symbol| world.contains(symbol)))
            })
            .map(|(index, _)| index)
            .collect()
    }

    // The values held by each coherence of the root frame in a kind, which is all a rule joining
    // several coherences can bind of it.
    fn shape(&self, kind: u32) -> Vec<Vec<Symbol>> {
        let (state, _) = self.taxonomy.kind(kind);
        state
            .world
            .iter()
            .filter(|world| world.frame == 0)
            .map(|world| world.particle.iter().map(|token| token.value).collect())
            .collect()
    }

    // The multisets of kinds, two or more parts, that some rule joining several coherences could
    // bind in a makeup, each with the number of ways to choose its copies. A rule binds a distinct
    // coherence for each input, and a copy of a kind hosts more than one input only when it holds
    // more than one coherence of the root frame. Every multiset is kept once, since its table holds
    // the events of every rule that binds exactly those parts.
    fn candidate(&self, makeup: &Makeup) -> Vec<(Vec<u32>, u64)> {
        let present = run(&makeup.kind)
            .into_iter()
            .filter(|(kind, _)| {
                self.surface
                    .get(kind)
                    .is_some_and(|surface| !surface.is_empty())
            })
            .collect::<Vec<_>>();
        if present.is_empty() {
            return Vec::new();
        }
        let holds = |kind: u32, input: &[Symbol]| {
            self.surface[&kind]
                .iter()
                .any(|world| input.iter().all(|symbol| world.contains(symbol)))
        };
        let mut result = IndexMap::<Vec<u32>, u64, Builder>::default();
        for rule in &self.pattern {
            let option = rule
                .iter()
                .map(|input| {
                    present
                        .iter()
                        .filter(|(kind, _)| holds(*kind, input))
                        .copied()
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            if option.iter().any(Vec::is_empty) {
                continue;
            }
            let mut index = vec![0; option.len()];
            loop {
                let mut used = HashMap::<u32, (usize, usize), Builder>::default();
                for (input, &position) in index.iter().enumerate() {
                    let (kind, count) = option[input][position];
                    used.entry(kind).or_insert((0, count)).0 += 1;
                }
                let mut kinds = used.into_iter().collect::<Vec<_>>();
                kinds.sort_unstable_by_key(|(kind, _)| *kind);
                let range = kinds
                    .iter()
                    .map(|&(kind, (inputs, count))| {
                        let high = inputs.min(count);
                        let low = if self.surface[&kind].len() > 1 {
                            1
                        } else {
                            inputs
                        };
                        (low, high)
                    })
                    .collect::<Vec<_>>();
                if range.iter().all(|(low, high)| low <= high) {
                    let mut taken = range.iter().map(|&(low, _)| low).collect::<Vec<_>>();
                    loop {
                        if taken.iter().sum::<usize>() >= 2 {
                            let mut multiset = Vec::new();
                            let mut ways = 1;
                            for (position, &(kind, (_, count))) in kinds.iter().enumerate() {
                                multiset.extend(std::iter::repeat_n(kind, taken[position]));
                                ways *= choose(count, taken[position]);
                            }
                            result.insert(multiset, ways);
                        }
                        let Some(position) =
                            (0..taken.len()).find(|&position| taken[position] < range[position].1)
                        else {
                            break;
                        };
                        taken[position] += 1;
                        for (earlier, &(low, _)) in taken.iter_mut().zip(&range).take(position) {
                            *earlier = low;
                        }
                    }
                }
                let Some(input) =
                    (0..index.len()).find(|&input| index[input] + 1 < option[input].len())
                else {
                    break;
                };
                index[input] += 1;
                for earlier in index.iter_mut().take(input) {
                    *earlier = 0;
                }
            }
        }
        result.into_iter().collect()
    }

    // Every configuration an event leads to from a makeup, with how many distinct events lead
    // there that way.
    fn expand(&mut self, makeup: &Makeup) -> Result<Vec<(Makeup, u64)>, Unsupported> {
        self.prepare(makeup)?;
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
        for (multiset, ways) in self.candidate(makeup) {
            for entry in &self.join[&(makeup.root, multiset.clone())] {
                let changed = replace(&makeup.kind, &multiset, &entry.produced);
                result.push((
                    Makeup {
                        root: entry.root,
                        kind: changed,
                    },
                    ways * u64::from(entry.count),
                ));
            }
        }
        Ok(result)
    }

    // Explores every schedule of plain events breadth first, numbering configurations in the order
    // they are found, and stops short of any configuration or application the limits refuse.
    pub fn explore(&mut self, limit: Limit, cycle: Cycle) -> Result<Exploration, Unsupported> {
        let mut space = HashMap::<Makeup, usize, Builder>::default();
        space.insert(self.start.clone(), 0);
        let mut marking = vec![self.start.clone()];
        let mut outgoing = Vec::<Vec<usize>>::new();
        let mut event = 0;
        let mut end = Vec::new();
        let mut blocked = false;
        let mut next = 0;
        while next < marking.len() {
            let current = marking[next].clone();
            let successor = self.expand(&current)?;
            if successor.is_empty() {
                end.push(next);
            }
            let mut target = Vec::with_capacity(successor.len());
            for (makeup, count) in successor {
                let (coherence, occurrence, scope) = self.taxonomy.measure(&makeup);
                if !limit.admits(coherence, occurrence, scope) {
                    blocked = true;
                    continue;
                }
                let index = match space.get(&makeup) {
                    Some(&index) => index,
                    None if marking.len() >= limit.configuration => {
                        blocked = true;
                        continue;
                    }
                    None => {
                        space.insert(makeup.clone(), marking.len());
                        marking.push(makeup);
                        marking.len() - 1
                    }
                };
                event += count;
                target.push(index);
            }
            outgoing.push(target);
            next += 1;
        }
        let endless = (cycle == Cycle::Find).then(|| cyclic(&outgoing));
        Ok(Exploration {
            closed: !blocked,
            configuration: marking.len(),
            event,
            end: end
                .into_iter()
                .map(|index| Marking {
                    root: marking[index].root,
                    kind: marking[index].kind.clone(),
                })
                .collect(),
            endless,
        })
    }
}

impl Exploration {
    // Two explorations of one program's net agree number for number when they close alike, find
    // as many configurations and events, the same end configurations in the same order and, where
    // both looked for one, a cycle in both or neither; each net numbers kinds in the order it
    // grounded its parts, so ends are compared by their canonical states.
    pub fn agrees(&self, net: &Net, reference: &Self, theirs: &Net) -> Result<(), Disagreement> {
        if self.closed != reference.closed {
            return Err(Disagreement::Closed {
                reference: reference.closed,
                laser: self.closed,
            });
        }
        if let (Some(endless), Some(expected)) = (self.endless, reference.endless)
            && endless != expected
        {
            return Err(Disagreement::Endless {
                reference: expected,
                laser: endless,
            });
        }
        if self.configuration != reference.configuration {
            return Err(Disagreement::Configuration {
                missing: reference.configuration.saturating_sub(self.configuration),
                extra: self.configuration.saturating_sub(reference.configuration),
            });
        }
        if self.event != reference.event {
            return Err(Disagreement::Event {
                missing: reference.event.saturating_sub(self.event) as usize,
                extra: self.event.saturating_sub(reference.event) as usize,
            });
        }
        let wanted = reference
            .end
            .iter()
            .map(|marking| theirs.state(marking))
            .collect::<Vec<_>>();
        let found = self
            .end
            .iter()
            .map(|marking| net.state(marking))
            .collect::<Vec<_>>();
        if wanted != found {
            return Err(Disagreement::Configuration {
                missing: wanted.iter().filter(|state| !found.contains(state)).count(),
                extra: found.iter().filter(|state| !wanted.contains(state)).count(),
            });
        }
        Ok(())
    }

    // A net's exploration mirrors the plain engine's when both close with the same number of
    // configurations and events, the same end configurations and a cycle in both or neither.
    pub fn mirrors(&self, net: &Net, plain: &Laser) -> Result<(), Disagreement> {
        let summary = plain.summary();
        if self.closed != summary.closed || !self.closed {
            return Err(Disagreement::Closed {
                reference: summary.closed,
                laser: self.closed,
            });
        }
        let ending = plain.ending();
        if let Some(endless) = self.endless
            && endless != ending.endless
        {
            return Err(Disagreement::Endless {
                reference: ending.endless,
                laser: endless,
            });
        }
        if self.configuration != summary.state {
            return Err(Disagreement::Configuration {
                missing: summary.state.saturating_sub(self.configuration),
                extra: self.configuration.saturating_sub(summary.state),
            });
        }
        let expected = summary.event as u64;
        if self.event != expected {
            return Err(Disagreement::Event {
                missing: expected.saturating_sub(self.event) as usize,
                extra: self.event.saturating_sub(expected) as usize,
            });
        }
        let mut wanted = ending
            .end
            .iter()
            .map(|&index| plain.state[index].canonical().state)
            .collect::<Vec<_>>();
        let mut found = self
            .end
            .iter()
            .map(|marking| net.state(marking))
            .collect::<Vec<_>>();
        wanted.sort();
        found.sort();
        if wanted != found {
            return Err(Disagreement::Configuration {
                missing: wanted.iter().filter(|state| !found.contains(state)).count(),
                extra: found.iter().filter(|state| !wanted.contains(state)).count(),
            });
        }
        Ok(())
    }
}

fn cyclic(outgoing: &[Vec<usize>]) -> bool {
    let mut color = vec![0u8; outgoing.len()];
    if outgoing.is_empty() {
        return false;
    }
    let mut stack = vec![(0, 0)];
    color[0] = 1;
    while let Some(&mut (node, ref mut position)) = stack.last_mut() {
        let Some(&next) = outgoing[node].get(*position) else {
            color[node] = 2;
            stack.pop();
            continue;
        };
        *position += 1;
        match color[next] {
            0 => {
                color[next] = 1;
                stack.push((next, 0));
            }
            1 => return true,
            _ => {}
        }
    }
    false
}
