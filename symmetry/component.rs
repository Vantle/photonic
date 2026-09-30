use crate::graph::{Color, Edge, Graph};
use crate::partition::Partition;
use crate::search::{Exhausted, Labeling, plain};
use code::forest::Forest;
use hashing::Builder;
use std::collections::{BTreeSet, HashMap};

const NONE: u32 = u32::MAX;

// Joining the vertices of a component the path has entered takes the automorphisms that fix the
// path's vertices in it: a group this small is listed whole, a larger one only by its generators,
// which still join soundly but may leave some of its orbits for the search to discover.
const ELEMENT: usize = 64;

struct Family {
    member: Vec<u32>,
    orbit: Vec<u32>,
    generator: Vec<Vec<u32>>,
    automorphism: Vec<Vec<u32>>,
}

struct Component {
    family: u32,
    label: Vec<u32>,
}

struct Labeled {
    label: Vec<u32>,
    certificate: Vec<u64>,
    generator: Vec<Vec<u32>>,
}

// The connected components of a graph without its roots, canonically labeled where they could be
// interchangeable, so the search knows from the start every automorphism that moves only
// components the path has not entered instead of finding them one leaf at a time.
#[derive(Default)]
pub(crate) struct Decomposition {
    component: Vec<Component>,
    family: Vec<Family>,
    owner: Vec<u32>,
    position: Vec<u32>,
    pub node: usize,
}

fn root(graph: &Graph, vertex: usize) -> bool {
    matches!(graph.color[vertex], Color::Part(_))
}

fn group(graph: &Graph) -> Vec<Vec<u32>> {
    let mut forest = Forest::new(graph.len());
    for vertex in (0..graph.len()).filter(|&vertex| !root(graph, vertex)) {
        for edge in &graph.child[vertex] {
            forest.join(vertex, edge.vertex as usize);
        }
    }
    forest
        .group()
        .into_iter()
        .filter(|member| !root(graph, member[0]))
        .map(|member| member.into_iter().map(|vertex| vertex as u32).collect())
        .collect()
}

// A component keeps a copy of each root it hangs from, holding only the edges into it, which the
// component's parent edges already list, so building every component reads each edge once.
fn subgraph(graph: &Graph, member: &[u32]) -> (Graph, Vec<u32>) {
    let attached = member
        .iter()
        .flat_map(|&vertex| &graph.parent[vertex as usize])
        .map(|edge| edge.vertex)
        .filter(|&parent| root(graph, parent as usize))
        .collect::<BTreeSet<_>>();
    let origin = member.iter().copied().chain(attached).collect::<Vec<_>>();
    let local = origin
        .iter()
        .enumerate()
        .map(|(index, &vertex)| (vertex, index as u32))
        .collect::<HashMap<_, _, Builder>>();
    let mut child = member
        .iter()
        .map(|&vertex| {
            graph.child[vertex as usize]
                .iter()
                .map(|edge| Edge {
                    vertex: local[&edge.vertex],
                    ..*edge
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    child.resize(origin.len(), Vec::new());
    for (index, &vertex) in member.iter().enumerate() {
        for edge in graph.parent[vertex as usize]
            .iter()
            .filter(|edge| root(graph, edge.vertex as usize))
        {
            child[local[&edge.vertex] as usize].push(Edge {
                vertex: index as u32,
                ..*edge
            });
        }
    }
    for edge in &mut child {
        edge.sort_unstable();
    }
    let color = origin
        .iter()
        .map(|&vertex| graph.color[vertex as usize])
        .collect();
    let atom = member
        .iter()
        .filter(|&&vertex| (vertex as usize) < graph.atom)
        .count();
    (Graph::link(color, child, atom), origin)
}

fn dense(size: usize, pair: &[(u32, u32)]) -> Vec<u32> {
    let mut map = (0..size as u32).collect::<Vec<_>>();
    for &(from, to) in pair {
        map[from as usize] = to;
    }
    map
}

fn span(generator: &[Vec<u32>], size: usize) -> Option<Vec<Vec<u32>>> {
    let identity = (0..size as u32).collect::<Vec<_>>();
    let mut known = BTreeSet::from([identity.clone()]);
    let mut frontier = vec![identity.clone()];
    while let Some(current) = frontier.pop() {
        for generator in generator {
            let next = current
                .iter()
                .map(|&position| generator[position as usize])
                .collect::<Vec<_>>();
            if known.contains(&next) {
                continue;
            }
            if known.len() > ELEMENT {
                return None;
            }
            known.insert(next.clone());
            frontier.push(next);
        }
    }
    known.remove(&identity);
    Some(known.into_iter().collect())
}

fn label(graph: &Graph, member: &[u32], budget: usize) -> Result<(Labeled, usize), Exhausted> {
    let (subgraph, origin) = subgraph(graph, member);
    let Labeling {
        element,
        certificate,
        generator,
        node,
        ..
    } = plain(&subgraph, budget)?;
    let mut position = vec![0; element.len()];
    for (place, &vertex) in element.iter().enumerate() {
        position[vertex as usize] = place as u32;
    }
    let generator = generator
        .iter()
        .map(|pair| {
            let map = dense(element.len(), pair);
            element
                .iter()
                .map(|&vertex| position[map[vertex as usize] as usize])
                .collect()
        })
        .collect();
    let label = element
        .iter()
        .map(|&vertex| {
            if (vertex as usize) < member.len() {
                return origin[vertex as usize];
            }
            NONE
        })
        .collect();
    let labeled = Labeled {
        label,
        certificate,
        generator,
    };
    Ok((labeled, node))
}

impl Family {
    fn new(member: Vec<u32>, generator: Vec<Vec<u32>>, size: usize) -> Self {
        let mut forest = Forest::new(size);
        for map in &generator {
            for (position, &image) in map.iter().enumerate() {
                forest.join(position, image as usize);
            }
        }
        let orbit = (0..size)
            .map(|position| forest.root(position) as u32)
            .collect();
        let automorphism = span(&generator, size).unwrap_or_else(|| generator.clone());
        Self {
            member,
            orbit,
            generator,
            automorphism,
        }
    }
}

impl Decomposition {
    pub(crate) fn new(graph: &Graph, partition: &Partition, budget: usize) -> Self {
        let group = group(graph);
        if group.len() < 2 {
            return Self::default();
        }
        let mut node = 0;
        let mut labeled = Vec::new();
        for member in group
            .iter()
            .filter(|member| member.iter().any(|&vertex| !partition.alone(vertex)))
        {
            match label(graph, member, budget.saturating_sub(node)) {
                Ok((value, spent)) => {
                    node += spent;
                    labeled.push(value);
                }
                Err(Exhausted::Node(spent)) => {
                    node += spent;
                    break;
                }
                Err(Exhausted::Depth(_)) => break,
            }
        }
        let mut form = HashMap::<&[u64], Vec<usize>, Builder>::default();
        for (index, value) in labeled.iter().enumerate() {
            form.entry(&value.certificate).or_default().push(index);
        }
        let mut kept = form
            .into_values()
            .filter(|member| member.len() > 1 || !labeled[member[0]].generator.is_empty())
            .collect::<Vec<_>>();
        kept.sort_unstable();
        let mut decomposition = Self {
            owner: vec![NONE; graph.len()],
            position: vec![NONE; graph.len()],
            node,
            ..Self::default()
        };
        for member in kept {
            let family = decomposition.family.len() as u32;
            let first = decomposition.component.len() as u32;
            for &index in &member {
                let owner = decomposition.component.len() as u32;
                for (place, &vertex) in labeled[index].label.iter().enumerate() {
                    if vertex != NONE {
                        decomposition.owner[vertex as usize] = owner;
                        decomposition.position[vertex as usize] = place as u32;
                    }
                }
                decomposition.component.push(Component {
                    family,
                    label: std::mem::take(&mut labeled[index].label),
                });
            }
            let representative = &mut labeled[member[0]];
            decomposition.family.push(Family::new(
                (first..first + member.len() as u32).collect(),
                std::mem::take(&mut representative.generator),
                decomposition.component[first as usize].label.len(),
            ));
        }
        decomposition
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.family.is_empty()
    }

    pub(crate) fn point(&self) -> Vec<Vec<u32>> {
        vec![Vec::new(); self.component.len()]
    }

    fn owner(&self, vertex: u32) -> Option<u32> {
        self.owner
            .get(vertex as usize)
            .copied()
            .filter(|&owner| owner != NONE)
    }

    pub(crate) fn enter(&self, point: &mut [Vec<u32>], vertex: u32) {
        if let Some(owner) = self.owner(vertex) {
            point[owner as usize].push(self.position[vertex as usize]);
        }
    }

    pub(crate) fn leave(&self, point: &mut [Vec<u32>], vertex: u32) {
        if let Some(owner) = self.owner(vertex) {
            point[owner as usize].pop();
        }
    }

    // Whether the components the path has not entered already join a whole cell, which decides
    // most nodes of a crowd of interchangeable components without building its orbits.
    pub(crate) fn whole(&self, point: &[Vec<u32>], cell: &[u32]) -> bool {
        let key = |vertex: u32| {
            let owner = self.owner(vertex)?;
            if !point[owner as usize].is_empty() {
                return None;
            }
            let family = self.component[owner as usize].family;
            Some((
                family,
                self.family[family as usize].orbit[self.position[vertex as usize] as usize],
            ))
        };
        let Some(first) = key(cell[0]) else {
            return false;
        };
        cell[1..].iter().all(|&vertex| key(vertex) == Some(first))
    }

    // Two vertices of components the path has not entered join when an automorphism of their
    // family's form maps one's position to the other's, through the swap of their components when
    // they differ; a vertex of an entered component joins its image under each automorphism of its
    // family that fixes every position the path holds in that component. Each such map is an
    // automorphism of the whole graph that fixes the path, which is all orbit pruning needs.
    pub(crate) fn join(
        &self,
        point: &[Vec<u32>],
        cell: &[u32],
        local: impl Fn(u32) -> usize,
        forest: &mut Forest,
    ) {
        if self.is_empty() {
            return;
        }
        let mut first = HashMap::<(u32, u32), usize, Builder>::default();
        let mut inside = HashMap::<u32, Vec<u32>, Builder>::default();
        for &vertex in cell {
            let Some(owner) = self.owner(vertex) else {
                continue;
            };
            if !point[owner as usize].is_empty() {
                inside.entry(owner).or_default().push(vertex);
                continue;
            }
            let family = self.component[owner as usize].family;
            let orbit = self.family[family as usize].orbit[self.position[vertex as usize] as usize];
            let index = local(vertex);
            match first.get(&(family, orbit)) {
                Some(&known) => forest.join(known, index),
                None => {
                    first.insert((family, orbit), index);
                }
            }
        }
        for (owner, member) in inside {
            let component = &self.component[owner as usize];
            let point = &point[owner as usize];
            for automorphism in &self.family[component.family as usize].automorphism {
                if point
                    .iter()
                    .any(|&position| automorphism[position as usize] != position)
                {
                    continue;
                }
                for &vertex in &member {
                    let image = automorphism[self.position[vertex as usize] as usize];
                    forest.join(local(vertex), local(component.label[image as usize]));
                }
            }
        }
    }

    // Swaps of neighbouring members generate every permutation of a family, and the generators of
    // its form, carried by its first member, then reach every member through them.
    pub(crate) fn generator(&self) -> Vec<Vec<(u32, u32)>> {
        let moved = |from: &[u32], to: &[u32]| {
            from.iter()
                .zip(to)
                .filter(|&(&left, &right)| left != right && left != NONE)
                .map(|(&left, &right)| (left, right))
                .collect::<Vec<_>>()
        };
        let mut result = Vec::new();
        for family in &self.family {
            let label = |member: u32| self.component[member as usize].label.as_slice();
            for pair in family.member.windows(2) {
                let (left, right) = (label(pair[0]), label(pair[1]));
                let mut swap = moved(left, right);
                swap.extend(moved(right, left));
                result.push(swap);
            }
            let own = label(family.member[0]);
            for generator in &family.generator {
                let image = generator
                    .iter()
                    .map(|&position| own[position as usize])
                    .collect::<Vec<_>>();
                result.push(moved(own, &image));
            }
        }
        result
    }
}
