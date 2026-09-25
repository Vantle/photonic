use crate::group::{Permutation, Size};
use crate::search::Exhausted;
use crate::structure::{Structure, Symmetry};
use code::atom::Atom;

pub struct Class {
    pub member: Vec<usize>,
    pub atom: Vec<Vec<Atom>>,
    pub form: Structure,
    pub size: Size,
    pub block: Vec<Vec<Atom>>,
    pub generator: Vec<Permutation>,
}

fn class(member: Vec<usize>, structure: &[Structure], symmetry: &[Symmetry]) -> Class {
    let first = &symmetry[member[0]];
    let canonical = first.canonical();
    let map = |atom: Atom| canonical[&atom];
    let renaming = member
        .iter()
        .map(|&index| {
            first
                .isomorphism(&symmetry[index])
                .expect("members of a class share one key")
        })
        .collect::<Vec<_>>();
    Class {
        atom: first
            .atom
            .iter()
            .map(|atom| renaming.iter().map(|map| map[atom]).collect())
            .collect(),
        form: first.form(&structure[member[0]]),
        size: first.size.clone(),
        block: first
            .block
            .iter()
            .map(|block| block.iter().map(|&atom| map(atom)).collect())
            .collect(),
        generator: first
            .generator
            .iter()
            .map(|permutation| permutation.rename(map))
            .collect(),
        member,
    }
}

pub fn compare(structure: &[Structure], budget: usize) -> Result<Vec<Class>, Exhausted> {
    let symmetry = structure
        .iter()
        .map(|structure| structure.symmetry(budget))
        .collect::<Result<Vec<_>, _>>()?;
    let mut member: Vec<Vec<usize>> = Vec::new();
    for index in 0..symmetry.len() {
        match member
            .iter_mut()
            .find(|entry| symmetry[entry[0]].key == symmetry[index].key)
        {
            Some(entry) => entry.push(index),
            None => member.push(vec![index]),
        }
    }
    Ok(member
        .into_iter()
        .map(|member| class(member, structure, &symmetry))
        .collect())
}
