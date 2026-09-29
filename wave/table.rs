use crate::hash;
use photonic::laser::net::{Entry, Marking, Net, Unsupported};
use std::collections::HashMap;

pub const EMPTY: u32 = u32::MAX;
pub const LONE: u32 = u32::MAX;

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

fn saturate(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

// The net's tables as the kernels read them. Each entry is its root, its count, the number of
// produced kinds, the two halves of the sum its root and produced kinds add to a marking's hash, the
// coherences, occurrences and frames its produced kinds hold, the kind it consumes or LONE, then
// the kinds; a part's entries lie
// together, and a table hashed by root and kind finds them. Kinds and roots are numbered by the net,
// so their sizes and join masks are arrays. Every part a marking holds is grounded before the
// marking is expanded, in the order the host's net grounds them, so both nets number kinds alike
// and a marking's runs of kinds come in the same order on both.
pub struct Table {
    pub entry: Vec<u32>,
    pub single: HashMap<(u32, u32), (u32, u32)>,
    pub lone: Vec<u32>,
    pub size: Vec<u32>,
    pub base: Vec<u32>,
    pub mask: Vec<u64>,
    pub need: Vec<u64>,
    pub wide: bool,
    pub dirty: bool,
}

impl Table {
    pub fn new(net: &Net) -> Self {
        let pattern = net.join();
        let input = pattern.iter().sum::<usize>();
        let wide = input > 64;
        let mut need = Vec::with_capacity(pattern.len());
        let mut next = 0;
        for count in pattern {
            let mut value = 0u64;
            for index in next..next + count {
                if index < 64 {
                    value |= 1 << index;
                }
            }
            next += count;
            need.push(value);
        }
        Self {
            entry: Vec::new(),
            single: HashMap::new(),
            lone: Vec::new(),
            size: Vec::new(),
            base: Vec::new(),
            mask: Vec::new(),
            need: if wide { Vec::new() } else { need },
            wide,
            dirty: true,
        }
    }

    // Every root and kind an entry leads to is numbered before a kernel can read its size.
    fn pack(&mut self, net: &mut Net, entry: &[Entry], consumed: u32) -> (u32, u32) {
        for value in entry {
            self.register(net, value.root, &value.produced);
        }
        let start = saturate(self.entry.len());
        for value in entry {
            let gain = value
                .produced
                .iter()
                .fold(hash::head(value.root), |sum, &kind| {
                    sum.wrapping_add(hash::term(kind))
                });
            let [world, occurrence, frame] = self.measure(&value.produced);
            self.entry.extend([
                value.root,
                value.count,
                saturate(value.produced.len()),
                gain as u32,
                (gain >> 32) as u32,
                world,
                occurrence,
                frame,
                consumed,
            ]);
            self.entry.extend(&value.produced);
        }
        self.dirty = true;
        (start, saturate(entry.len()))
    }

    fn register(&mut self, net: &mut Net, root: u32, kind: &[u32]) {
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
        while self.mask.len() < top {
            let next = saturate(self.mask.len());
            let size = net.size(next);
            self.size
                .extend([size.world, size.occurrence, size.frame].map(saturate));
            let reach = net.reach(next);
            let mask = if self.wide {
                u64::from(!reach.is_empty())
            } else {
                reach.iter().fold(0, |mask, &index| mask | 1 << index)
            };
            self.mask.push(mask);
            self.dirty = true;
        }
    }

    // The coherences, occurrences and frames that components of these kinds hold together.
    pub fn measure(&self, kind: &[u32]) -> [u32; 3] {
        kind.iter().fold([0u32; 3], |sum, &value| {
            let size = &self.size[3 * value as usize..3 * value as usize + 3];
            [
                sum[0].saturating_add(size[0]),
                sum[1].saturating_add(size[1]),
                sum[2].saturating_add(size[2]),
            ]
        })
    }

    fn lone(&mut self, net: &mut Net, root: u32) -> Result<(), Unsupported> {
        self.register(net, root, &[]);
        if self.lone[2 * root as usize] != EMPTY {
            return Ok(());
        }
        let entry = net.lone(root)?.to_vec();
        let (start, number) = self.pack(net, &entry, LONE);
        self.lone[2 * root as usize] = start;
        self.lone[2 * root as usize + 1] = number;
        Ok(())
    }

    fn single(&mut self, net: &mut Net, root: u32, kind: u32) -> Result<(), Unsupported> {
        self.register(net, root, &[kind]);
        if self.single.contains_key(&(root, kind)) {
            return Ok(());
        }
        let entry = net.single(root, kind)?.to_vec();
        let location = self.pack(net, &entry, kind);
        self.single.insert((root, kind), location);
        Ok(())
    }

    // Numbers the root and kinds of a marking the host made, so its sizes are known.
    pub fn know(&mut self, net: &mut Net, marking: &Marking) {
        self.register(net, marking.root, &marking.kind);
    }

    // Grounds a marking as the host's net does when it expands it, then enters the parts it holds.
    pub fn prepare(&mut self, net: &mut Net, marking: &Marking) -> Result<(), Unsupported> {
        net.visit(marking)?;
        self.lone(net, marking.root)?;
        for &kind in &marking.kind {
            self.single(net, marking.root, kind)?;
        }
        Ok(())
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

    pub fn joining(&self) -> usize {
        if self.wide { 0 } else { self.need.len() }
    }
}
