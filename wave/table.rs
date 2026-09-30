use crate::failure::Failure;
use crate::hash;
use crate::setting::{EMPTY, HEADER, PART, saturate};
use hashing::Builder;
use photonic::laser::makeup::Makeup;
use photonic::laser::net::{Entry, Expansion, Net};
use std::collections::HashMap;

fn mix(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x85eb_ca6b);
    value ^= value >> 13;
    value = value.wrapping_mul(0xc2b2_ae35);
    value ^= value >> 16;
    value
}

fn position(root: u32, kind: u32, mask: usize) -> usize {
    mix(root.wrapping_mul(0x9e37_79b9) ^ mix(kind.wrapping_add(0x7f4a_7c15))) as usize & mask
}

// The net's tables as the kernels read them. Each entry is a header of HEADER words, its root, its
// count, the number of produced kinds, the two halves of what it adds to a marking's sum of kind
// terms once the terms of the kinds it consumes are taken away, the coherences, occurrences and
// frames its produced kinds hold and the number of kinds it consumes, then the produced kinds and
// the consumed ones from the largest down: none for the events binding only the root, the kind of a
// component, or the kinds of a part of several components. A part's entries lie together; a table hashed by root and
// kind finds a component's, and a catalog hashed by root and digest finds a part's. Kinds and roots
// are numbered by the net, so their sizes, their coherences of the root frame and where the inputs
// of joining rules they reach lie are arrays, as are each input's rule and each rule's number of
// inputs. Every part a marking holds is grounded before the marking is expanded, in the order the
// host's net grounds them, so both nets number kinds and parts alike and a marking's runs of kinds
// come in the same order on both.
pub struct Table {
    pub entry: Vec<u32>,
    pub single: HashMap<(u32, u32), (u32, u32), Builder>,
    pub lone: Vec<u32>,
    pub size: Vec<u32>,
    pub base: Vec<u32>,
    pub coherence: Vec<u32>,
    pub span: Vec<u32>,
    pub reach: Vec<u32>,
    pub rule: Vec<u32>,
    pub arity: Vec<u32>,
    pub part: Vec<u32>,
    pub member: Vec<u32>,
    pub dirty: bool,
}

impl Table {
    pub fn new(net: &Net) -> Self {
        let arity = net.arity();
        let rule = arity
            .iter()
            .enumerate()
            .flat_map(|(rule, &count)| std::iter::repeat_n(saturate(rule), count))
            .collect();
        Self {
            entry: Vec::new(),
            single: HashMap::default(),
            lone: Vec::new(),
            size: Vec::new(),
            base: Vec::new(),
            coherence: Vec::new(),
            span: Vec::new(),
            reach: Vec::new(),
            rule,
            arity: arity.into_iter().map(saturate).collect(),
            part: Vec::new(),
            member: Vec::new(),
            dirty: true,
        }
    }

    // Every root and kind an entry leads to is numbered before a kernel can read its size.
    fn pack(&mut self, net: &Net, entry: &[Entry], consumed: &[u32]) -> (u32, u32) {
        for value in entry {
            self.register(net, value.root, &value.produced);
        }
        let loss = consumed
            .iter()
            .fold(0u64, |sum, &kind| sum.wrapping_add(hash::term(kind)));
        let start = saturate(self.entry.len());
        for value in entry {
            let gain = hash::sum(value.root, &value.produced).wrapping_sub(loss);
            let [world, occurrence, frame] = self.measure(&value.produced);
            let header: [u32; HEADER] = [
                value.root,
                value.count,
                saturate(value.produced.len()),
                gain as u32,
                (gain >> 32) as u32,
                world,
                occurrence,
                frame,
                saturate(consumed.len()),
            ];
            self.entry.extend(header);
            self.entry.extend(&value.produced);
            self.entry.extend(consumed.iter().rev());
        }
        self.dirty = true;
        (start, saturate(entry.len()))
    }

    fn register(&mut self, net: &Net, root: u32, kind: &[u32]) {
        while self.base.len() <= root as usize {
            let next = saturate(self.base.len());
            self.base.push(saturate(net.occurrence(next)));
            self.lone.extend([EMPTY, 0]);
            self.dirty = true;
        }
        let top = kind
            .iter()
            .copied()
            .max()
            .map_or(0, |value| value as usize + 1);
        while self.coherence.len() < top {
            let next = saturate(self.coherence.len());
            let size = net.size(next);
            self.size
                .extend([size.world, size.occurrence, size.frame].map(saturate));
            let surface = net.shape(next);
            self.coherence.push(saturate(surface.coherence));
            self.span
                .extend([saturate(self.reach.len()), saturate(surface.reach.len())]);
            self.reach.extend(surface.reach.into_iter().map(saturate));
            self.dirty = true;
        }
    }

    // The coherences, occurrences and frames that components of these kinds hold together.
    fn measure(&self, kind: &[u32]) -> [u32; 3] {
        kind.iter().fold([0u32; 3], |sum, &value| {
            let size = &self.size[3 * value as usize..3 * value as usize + 3];
            [
                sum[0].saturating_add(size[0]),
                sum[1].saturating_add(size[1]),
                sum[2].saturating_add(size[2]),
            ]
        })
    }

    fn lone(&mut self, net: &Net, root: u32) -> Result<(), Failure> {
        self.register(net, root, &[]);
        if self.lone[2 * root as usize] != EMPTY {
            return Ok(());
        }
        let entry = net.lone(root).ok_or(Failure::Missing)?;
        let (start, number) = self.pack(net, entry, &[]);
        self.lone[2 * root as usize] = start;
        self.lone[2 * root as usize + 1] = number;
        Ok(())
    }

    fn single(&mut self, net: &Net, root: u32, kind: u32) -> Result<(), Failure> {
        self.register(net, root, &[kind]);
        if self.single.contains_key(&(root, kind)) {
            return Ok(());
        }
        let entry = net.single(root, kind).ok_or(Failure::Missing)?;
        let location = self.pack(net, entry, &[kind]);
        self.single.insert((root, kind), location);
        Ok(())
    }

    // Enters the parts of several components the net grounded since the last were entered, in the
    // order it grounded them, so the kernels number them as the net does.
    fn join(&mut self, net: &Net) {
        while let Some((part, entry)) = net.part(self.part.len() / PART) {
            let (start, number) = self.pack(net, entry, &part.kind);
            let first = saturate(self.member.len());
            self.member.extend(&part.kind);
            self.part
                .extend([part.root, first, saturate(part.kind.len()), start, number]);
        }
    }

    // Numbers the root and kinds of a marking the host made, so its sizes are known.
    pub fn know(&mut self, net: &Net, marking: &Makeup) {
        self.register(net, marking.root, &marking.kind);
    }

    // Visits a marking as the host's net does when it expands it, enters the parts it holds and
    // gives the successors of its events joining several components, with the work its host join
    // took; none when grounding its parts or its host join would take the net's work past the
    // allowance.
    pub fn prepare(
        &mut self,
        net: &mut Net,
        marking: &Makeup,
        allowance: usize,
    ) -> Result<Option<Expansion>, Failure> {
        let Some(expansion) = net.visit(marking, allowance)? else {
            return Ok(None);
        };
        self.lone(net, marking.root)?;
        for run in marking.kind.chunk_by(PartialEq::eq) {
            self.single(net, marking.root, run[0])?;
        }
        self.join(net);
        Ok(Some(expansion))
    }

    // The hashed index from a root and a kind to its entries, with its number of slots: four words a
    // slot, the root, the kind, the first entry and how many, with EMPTY roots marking free slots.
    pub fn key(&self) -> (Vec<u32>, usize) {
        let capacity = (2 * self.single.len()).max(16).next_power_of_two();
        let mask = capacity - 1;
        let mut key = vec![EMPTY; 4 * capacity];
        for (&(root, kind), &(start, number)) in &self.single {
            let mut slot = position(root, kind, mask);
            while key[4 * slot] != EMPTY {
                slot = (slot + 1) & mask;
            }
            key[4 * slot..4 * slot + 4].copy_from_slice(&[root, kind, start, number]);
        }
        (key, capacity)
    }

    // The hashed index from a part's root and digest to its number, with its number of slots: four
    // words a slot, the root, the two halves of the digest and the number, with EMPTY roots marking
    // free slots. A digest is a marking's hash of the part's root and kinds.
    pub fn catalog(&self) -> (Vec<u32>, usize) {
        let count = self.part.len() / PART;
        let capacity = (2 * count).max(16).next_power_of_two();
        let mask = capacity - 1;
        let mut catalog = vec![EMPTY; 4 * capacity];
        for (id, &[root, first, length, _, _]) in self.part.as_chunks::<PART>().0.iter().enumerate()
        {
            let (first, length) = (first as usize, length as usize);
            let digest = hash::digest(root, &self.member[first..first + length]);
            let mut slot = digest as usize & mask;
            while catalog[4 * slot] != EMPTY {
                slot = (slot + 1) & mask;
            }
            catalog[4 * slot..4 * slot + 4].copy_from_slice(&[
                root,
                digest as u32,
                (digest >> 32) as u32,
                saturate(id),
            ]);
        }
        (catalog, capacity)
    }
}
