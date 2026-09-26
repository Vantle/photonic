use crate::structure::Part;
use code::atom::Atom;
use code::output::Output;
use code::particle::Particle;
use code::rule::Rule;
use code::value::Value;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Relation {
    Member,
    Input,
    Output,
    Product,
    Body,
    Rule,
    Coherence,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Edge {
    pub relation: Relation,
    pub vertex: u32,
    pub count: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Color {
    Atom(u32, u32),
    Particle,
    Rule,
    Output,
    Part(u32),
}

impl Color {
    pub fn code(self) -> [u64; 2] {
        match self {
            Self::Atom(pin, size) => [0, u64::from(pin) << 32 | u64::from(size)],
            Self::Particle => [1, 0],
            Self::Rule => [2, 0],
            Self::Output => [3, 0],
            Self::Part(role) => [4, u64::from(role)],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Graph {
    pub color: Vec<Color>,
    pub child: Vec<Vec<Edge>>,
    pub parent: Vec<Vec<Edge>>,
    pub atom: usize,
}

struct Builder<'atom> {
    atom: &'atom HashMap<Atom, u32>,
    color: Vec<Color>,
    child: Vec<Vec<Edge>>,
    intern: HashMap<(Color, Vec<Edge>), u32>,
}

fn run<Item: PartialEq>(item: &[Item]) -> impl Iterator<Item = (&Item, u32)> {
    item.chunk_by(|left, right| left == right)
        .map(|chunk| (&chunk[0], chunk.len() as u32))
}

impl Builder<'_> {
    fn intern(&mut self, color: Color, mut child: Vec<Edge>) -> u32 {
        child.sort_unstable();
        let key = (color, child);
        if let Some(&vertex) = self.intern.get(&key) {
            return vertex;
        }
        let vertex = self.color.len() as u32;
        self.color.push(color);
        self.child.push(key.1.clone());
        self.intern.insert(key, vertex);
        vertex
    }

    fn value(&mut self, value: &Value) -> u32 {
        match value {
            Value::Atom(atom) => self.atom[atom],
            Value::Rule(rule) => self.rule(rule),
        }
    }

    fn particle(&mut self, particle: &Particle) -> u32 {
        let child = run(particle.value())
            .map(|(value, count)| Edge {
                relation: Relation::Member,
                vertex: self.value(value),
                count,
            })
            .collect();
        self.intern(Color::Particle, child)
    }

    fn output(&mut self, output: &Output) -> u32 {
        let mut child = vec![Edge {
            relation: Relation::Product,
            vertex: self.particle(output.particle()),
            count: 1,
        }];
        for (rule, count) in run(output.body().unwrap_or_default()) {
            child.push(Edge {
                relation: Relation::Body,
                vertex: self.rule(rule),
                count,
            });
        }
        self.intern(Color::Output, child)
    }

    fn rule(&mut self, rule: &Rule) -> u32 {
        let mut child = Vec::new();
        for (particle, count) in run(rule.input()) {
            child.push(Edge {
                relation: Relation::Input,
                vertex: self.particle(particle),
                count,
            });
        }
        for (output, count) in run(rule.output()) {
            child.push(Edge {
                relation: Relation::Output,
                vertex: self.output(output),
                count,
            });
        }
        self.intern(Color::Rule, child)
    }

    fn part(&mut self, role: u32, part: &Part) {
        let mut child = Vec::new();
        for (rule, count) in run(part.program.rule()) {
            child.push(Edge {
                relation: Relation::Rule,
                vertex: self.rule(rule),
                count,
            });
        }
        for (particle, count) in run(part.configuration.coherence()) {
            child.push(Edge {
                relation: Relation::Coherence,
                vertex: self.particle(particle),
                count,
            });
        }
        child.sort_unstable();
        self.color.push(Color::Part(role));
        self.child.push(child);
    }
}

impl Graph {
    pub fn new<'part>(
        atom: &[Atom],
        pin: &[Atom],
        part: impl IntoIterator<Item = (u32, &'part Part)>,
    ) -> Self {
        let index = atom
            .iter()
            .enumerate()
            .map(|(vertex, &atom)| (atom, vertex as u32))
            .collect::<HashMap<_, _>>();
        let mut builder = Builder {
            atom: &index,
            color: atom
                .iter()
                .map(|atom| {
                    Color::Atom(
                        pin.iter()
                            .position(|pinned| pinned == atom)
                            .map_or(0, |position| position as u32 + 1),
                        1,
                    )
                })
                .collect(),
            child: vec![Vec::new(); atom.len()],
            intern: HashMap::new(),
        };
        for (role, part) in part {
            builder.part(role, part);
        }
        Self::link(builder.color, builder.child, atom.len())
    }

    pub fn link(color: Vec<Color>, child: Vec<Vec<Edge>>, atom: usize) -> Self {
        let mut parent = vec![Vec::new(); color.len()];
        for (vertex, edge) in child.iter().enumerate() {
            for edge in edge {
                parent[edge.vertex as usize].push(Edge {
                    relation: edge.relation,
                    vertex: vertex as u32,
                    count: edge.count,
                });
            }
        }
        Self {
            color,
            child,
            parent,
            atom,
        }
    }

    pub fn len(&self) -> usize {
        self.color.len()
    }
}
