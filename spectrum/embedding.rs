use crate::configuration::{Configuration, Value};
use crate::failure::{Code, Failure};
use crate::pattern::{self, Body, Canon, Item};

// Patterns are small and the search prunes every place that cannot complete, so it visits a few
// places for each scope; the budget bounds patterns written to defeat the pruning.
const BUDGET: usize = 10_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Match {
    pub coherence: usize,
    pub occurrence: Vec<usize>,
}

// Where a pattern's parts went: each coherence part, the top level's first and then each scope's in
// the order a depth-first walk meets them, and each scope's frame in that order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Embedding {
    pub coherence: Vec<Match>,
    pub frame: Vec<usize>,
}

// The search for a place for every part took more steps than its budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Exhausted;

impl From<Exhausted> for Failure {
    fn from(_: Exhausted) -> Self {
        Self::new(
            Code::Pattern,
            "the pattern's scopes can be placed in too many ways to search; write fewer of them, or tell them apart",
        )
    }
}

// A scope of the pattern, in the order a depth-first walk meets them, with the scope it sits in and
// the nearest earlier scope beside it that is written alike.
struct Scope<'body> {
    body: &'body Body,
    parent: Option<usize>,
    twin: Option<usize>,
}

// A coherence part of the pattern, with the scope it sits in.
struct Part<'body> {
    particle: &'body [Item],
    scope: Option<usize>,
}

struct Search<'body, 'entry> {
    entry: &'entry Configuration,
    scope: Vec<Scope<'body>>,
    part: Vec<Part<'body>>,
    fit: Vec<Vec<bool>>,
    cover: Vec<Vec<Option<Vec<usize>>>>,
    image: Vec<Option<usize>>,
    taken: Vec<bool>,
    remaining: usize,
}

fn flatten<'body>(
    body: &'body Body,
    parent: Option<usize>,
    scope: &mut Vec<Scope<'body>>,
    part: &mut Vec<Part<'body>>,
) {
    part.extend(body.coherence.iter().map(|particle| Part {
        particle,
        scope: parent,
    }));
    for inner in &body.scope {
        let twin = (0..scope.len())
            .rev()
            .find(|&index| scope[index].parent == parent && scope[index].body == inner);
        scope.push(Scope {
            body: inner,
            parent,
            twin,
        });
        flatten(inner, Some(scope.len() - 1), scope, part);
    }
}

fn augment(
    row: usize,
    allow: &impl Fn(usize, usize) -> bool,
    owner: &mut [Option<usize>],
    seen: &mut [bool],
) -> bool {
    for column in 0..owner.len() {
        if seen[column] || !allow(row, column) {
            continue;
        }
        seen[column] = true;
        if owner[column].is_none_or(|other| augment(other, allow, owner, seen)) {
            owner[column] = Some(row);
            return true;
        }
    }
    false
}

// Whether every row can take a different column that allows it.
fn perfect(row: usize, column: usize, allow: impl Fn(usize, usize) -> bool) -> bool {
    let mut owner = vec![None; column];
    (0..row).all(|index| augment(index, &allow, &mut owner, &mut vec![false; column]))
}

// Each row takes the first column that leaves every later row a different column, the assignment a
// depth-first search finds first, without its factorial cost.
fn first(row: usize, column: usize, allow: impl Fn(usize, usize) -> bool) -> Option<Vec<usize>> {
    let mut taken = vec![false; column];
    let mut chosen = Vec::with_capacity(row);
    for index in 0..row {
        let mut found = None;
        for candidate in 0..column {
            if taken[candidate] || !allow(index, candidate) {
                continue;
            }
            taken[candidate] = true;
            let rest = perfect(row - index - 1, column, |later, target| {
                !taken[target] && allow(index + 1 + later, target)
            });
            if rest {
                found = Some(candidate);
                break;
            }
            taken[candidate] = false;
        }
        chosen.push(found?);
    }
    Some(chosen)
}

// Whether a scope's rules are live in a frame, each rule a different live rule.
fn live(body: &Body, entry: &Configuration, frame: usize, canon: &(impl Canon + ?Sized)) -> bool {
    let mut rule = entry.frame[frame]
        .rule
        .iter()
        .filter_map(|occurrence| match occurrence.value {
            Value::Rule(rule) => Some(rule),
            Value::Atom(_) => None,
        })
        .collect::<Vec<_>>();
    body.rule.iter().all(|definition| {
        rule.iter()
            .position(|&index| canon.canonical(index) == Some(definition))
            .map(|position| rule.swap_remove(position))
            .is_some()
    })
}

impl<'body, 'entry> Search<'body, 'entry> {
    fn new(body: &'body Body, entry: &'entry Configuration, canon: &(impl Canon + ?Sized)) -> Self {
        let mut scope = Vec::new();
        let mut part = Vec::new();
        flatten(body, None, &mut scope, &mut part);
        let cover = part
            .iter()
            .map(|part| {
                entry
                    .coherence
                    .iter()
                    .map(|coherence| pattern::cover(part.particle, coherence, canon))
                    .collect()
            })
            .collect::<Vec<Vec<_>>>();
        let frame = entry.frame.len();
        let mut fit = vec![vec![false; frame]; scope.len()];
        for index in (0..scope.len()).rev() {
            let own = (0..part.len())
                .filter(|&position| part[position].scope == Some(index))
                .collect::<Vec<_>>();
            let child = (index + 1..scope.len())
                .filter(|&other| scope[other].parent == Some(index))
                .collect::<Vec<_>>();
            for place in 1..frame {
                let fits = live(scope[index].body, entry, place, canon)
                    && perfect(own.len(), entry.coherence.len(), |row, coherence| {
                        entry.coherence[coherence].frame == place
                            && cover[own[row]][coherence].is_some()
                    })
                    && perfect(child.len(), frame, |row, inner| {
                        entry.frame[inner].parent == Some(place) && fit[child[row]][inner]
                    });
                fit[index][place] = fits;
            }
        }
        Self {
            entry,
            image: vec![None; scope.len()],
            taken: vec![false; frame],
            scope,
            part,
            fit,
            cover,
            remaining: BUDGET,
        }
    }

    // Whether a scope can still take a frame: a free one its parts fit, inside the frame its parent
    // took, if its parent took one.
    fn open(&self, scope: usize, frame: usize) -> bool {
        if self.taken[frame] || !self.fit[scope][frame] {
            return false;
        }
        match self.scope[scope]
            .parent
            .and_then(|parent| self.image[parent])
        {
            Some(parent) => self.entry.frame[frame].parent == Some(parent),
            None => true,
        }
    }

    // Whether a coherence part can still take a coherence: one it covers, in the frame its scope took
    // or, while its scope has none, in any frame the scope can still take.
    fn allowed(&self, part: usize, coherence: usize) -> bool {
        if self.cover[part][coherence].is_none() {
            return false;
        }
        let frame = self.entry.coherence[coherence].frame;
        match self.part[part].scope {
            None => true,
            Some(scope) => {
                self.image[scope].map_or_else(|| self.open(scope, frame), |image| image == frame)
            }
        }
    }

    // Whether the scopes still to place can each take a different frame, and every coherence part a
    // different coherence. Once every scope has its frame this is exact; before, it only prunes.
    fn feasible(&self) -> bool {
        let free = (0..self.scope.len())
            .filter(|&scope| self.image[scope].is_none())
            .collect::<Vec<_>>();
        perfect(free.len(), self.entry.frame.len(), |row, frame| {
            self.open(free[row], frame)
        }) && perfect(
            self.part.len(),
            self.entry.coherence.len(),
            |row, coherence| self.allowed(row, coherence),
        )
    }

    // Scopes take frames in the order a depth-first walk meets them, and a scope written like an
    // earlier one beside it takes a later frame, since swapping the two finds nothing new.
    fn place(&mut self, scope: usize) -> Result<bool, Exhausted> {
        self.remaining = self.remaining.checked_sub(1).ok_or(Exhausted)?;
        if !self.feasible() {
            return Ok(false);
        }
        if scope == self.scope.len() {
            return Ok(true);
        }
        let floor = self.scope[scope]
            .twin
            .and_then(|twin| self.image[twin])
            .map_or(1, |frame| frame + 1);
        for frame in floor..self.entry.frame.len() {
            if !self.open(scope, frame) {
                continue;
            }
            self.image[scope] = Some(frame);
            self.taken[frame] = true;
            if self.place(scope + 1)? {
                return Ok(true);
            }
            self.image[scope] = None;
            self.taken[frame] = false;
        }
        Ok(false)
    }

    fn embedding(&self) -> Option<Embedding> {
        let chosen = first(
            self.part.len(),
            self.entry.coherence.len(),
            |row, coherence| self.allowed(row, coherence),
        )?;
        Some(Embedding {
            coherence: chosen
                .into_iter()
                .enumerate()
                .map(|(part, coherence)| Match {
                    coherence,
                    occurrence: self.cover[part][coherence].clone().unwrap_or_default(),
                })
                .collect(),
            frame: self.image.iter().flatten().copied().collect(),
        })
    }
}

// A pattern's parts are found anywhere, a coherence in any scope and a scope at any depth, and a
// scope's parts inside the frame it takes; parts listed anywhere in the pattern take different
// coherences and frames, so no part of a configuration serves two, and the order parts are written
// in never matters.
pub fn assign(
    body: &Body,
    entry: &Configuration,
    canon: &(impl Canon + ?Sized),
) -> Result<Option<Embedding>, Exhausted> {
    if !body.rule.is_empty() {
        return Ok(None);
    }
    let mut search = Search::new(body, entry, canon);
    if !search.place(0)? {
        return Ok(None);
    }
    Ok(search.embedding())
}
