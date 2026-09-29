use super::ending;
use super::layout::Layout;
use super::pool::Pool;
use super::scan;
use super::taxonomy::{Draft, Makeup, Taxonomy};
use super::trace::Trace;
use super::transition;
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

    // Every distinct plain event of the part holding the root and these kinds that involves every
    // component of it, as the plain engines identify and apply it, grouped by what it makes of the
    // part; an event involving fewer components is an event of a smaller part, grounded with it.
    fn ground(&mut self, root: u32, kind: &[u32]) -> Result<Vec<Entry>, Unsupported> {
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
            let draft = self.taxonomy.analyze(&applied.state);
            if matches!(draft, Draft::Whole(_)) {
                return Err(Unsupported::Whole);
            }
            let (root, mut produced) = self.taxonomy.intern(&draft);
            produced.sort_unstable();
            effect.push((root, produced));
        }
        Ok(group(effect))
    }

    // The events that bind nothing but the root, once a visit grounded them.
    pub fn lone(&self, root: u32) -> Option<&[Entry]> {
        self.lone.get(&root).map(Vec::as_slice)
    }

    // The events of one component of a kind, with the root's rules, once a visit grounded them.
    pub fn single(&self, root: u32, kind: u32) -> Option<&[Entry]> {
        self.single.get(&(root, kind)).map(Vec::as_slice)
    }

    // Grounds every part a marking holds, and the multisets its kinds could join, as expanding the
    // marking does and in the same order, so a net that visits markings in the order this one
    // expands them numbers kinds as it does; then gives every configuration an event joining
    // several components leads to from the marking, in the order expanding it finds them.
    pub fn visit(&mut self, marking: &Marking) -> Result<Vec<Successor>, Unsupported> {
        self.prepare(marking.root, &marking.kind)?;
        Ok(self
            .joined(marking.root, &marking.kind)
            .into_iter()
            .map(|(makeup, count)| Successor {
                marking: Marking {
                    root: makeup.root,
                    kind: makeup.kind,
                },
                count,
            })
            .collect())
    }

    fn prepare(&mut self, root: u32, kind: &[u32]) -> Result<(), Unsupported> {
        if !self.lone.contains_key(&root) {
            let entry = self.ground(root, &[])?;
            self.lone.insert(root, entry);
        }
        for (value, _) in run(kind) {
            if !self.single.contains_key(&(root, value)) {
                let entry = self.ground(root, &[value])?;
                self.single.insert((root, value), entry);
            }
            self.survey(value);
        }
        for (multiset, _) in self.candidate(kind) {
            let key = (root, multiset);
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

    // Every configuration an event joining several components leads to from a makeup of the root
    // and these kinds, whose parts are grounded, with how many distinct events lead there that way.
    fn joined(&self, root: u32, kind: &[u32]) -> Vec<(Makeup, u64)> {
        let mut result = Vec::new();
        for (multiset, choice) in self.candidate(kind) {
            let key = (root, multiset);
            for entry in &self.join[&key] {
                result.push((
                    Makeup {
                        root: entry.root,
                        kind: replace(kind, &key.1, &entry.produced),
                    },
                    choice * u64::from(entry.count),
                ));
            }
        }
        result
    }

    // Whether the limits admit a marking's coherences, occurrences and scopes.
    pub fn admits(&self, marking: &Marking, limit: Limit) -> bool {
        let (coherence, occurrence, scope) = self.taxonomy.measure(&Makeup {
            root: marking.root,
            kind: marking.kind.clone(),
        });
        limit.admits(coherence, occurrence, scope)
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
    pub fn arity(&self) -> Vec<usize> {
        self.pattern.iter().map(Vec::len).collect()
    }

    // The inputs of the joining rules, numbered across rules in order, that a coherence of the root
    // frame in a component of this kind could fill.
    pub fn reach(&self, kind: u32) -> Vec<usize> {
        let surface = self.shape(kind);
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
    fn candidate(&self, kind: &[u32]) -> Vec<(Vec<u32>, u64)> {
        let present = run(kind)
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
                let mut chosen = used.into_iter().collect::<Vec<_>>();
                chosen.sort_unstable_by_key(|(kind, _)| *kind);
                let range = chosen
                    .iter()
                    .map(|&(kind, (filled, count))| {
                        let high = filled.min(count);
                        let low = if self.surface[&kind].len() > 1 {
                            1
                        } else {
                            filled
                        };
                        (low, high)
                    })
                    .collect::<Vec<_>>();
                if range.iter().all(|(low, high)| low <= high) {
                    let mut taken = range.iter().map(|&(low, _)| low).collect::<Vec<_>>();
                    loop {
                        if taken.iter().sum::<usize>() >= 2 {
                            let mut multiset = Vec::new();
                            let mut choice = 1;
                            for (position, &(kind, (_, count))) in chosen.iter().enumerate() {
                                multiset.extend(std::iter::repeat_n(kind, taken[position]));
                                choice *= choose(count, taken[position]);
                            }
                            result.insert(multiset, choice);
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
        self.prepare(makeup.root, &makeup.kind)?;
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
        result.extend(self.joined(makeup.root, &makeup.kind));
        Ok(result)
    }

    // Explores every schedule of plain events breadth first, numbering configurations in the order
    // they are found, and stops short of any configuration or application the limits refuse.
    // Looking for a run that goes on forever keeps every edge, and searches them only when one
    // leads to a configuration found no later than its source, since every cycle has such an edge.
    pub fn explore(&mut self, limit: Limit, cycle: Cycle) -> Result<Exploration, Unsupported> {
        let mut space = IndexSet::<Makeup, Builder>::default();
        space.insert(self.start.clone());
        let mut first = Vec::new();
        let mut edge = Vec::new();
        let mut backward = false;
        let mut event = 0;
        let mut end = Vec::new();
        let mut blocked = false;
        let mut next = 0;
        while next < space.len() {
            let successor = self.expand(&space[next])?;
            if successor.is_empty() {
                end.push(next);
            }
            if cycle == Cycle::Find {
                first.push(edge.len());
            }
            for (makeup, count) in successor {
                let (coherence, occurrence, scope) = self.taxonomy.measure(&makeup);
                if !limit.admits(coherence, occurrence, scope) {
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
            first.push(edge.len());
            backward
                && ending::cyclic(space.len(), |node| {
                    edge[first[node]..first[node + 1]].iter().copied()
                })
        });
        Ok(Exploration {
            closed: !blocked,
            configuration: space.len(),
            event,
            end: end
                .into_iter()
                .map(|index| Marking {
                    root: space[index].root,
                    kind: space[index].kind.clone(),
                })
                .collect(),
            endless,
        })
    }
}
