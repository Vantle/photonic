mod emit;
mod reduce;

use crate::address::Address;
use crate::failure::Failure;
use crate::format::Format;
use crate::gate::assignment;
use crate::gate::{Gate, Kind};

struct Circuit {
    gate: Vec<Gate>,
    initial: Vec<(usize, u8)>,
    domain: Vec<Vec<u8>>,
    radix: u8,
}

impl Circuit {
    fn append(&mut self, input: Vec<usize>, kind: Kind) -> Vec<usize> {
        let domain = input
            .iter()
            .map(|&wire| self.domain[wire].clone())
            .collect::<Vec<_>>();
        let table = assignment(&domain)
            .iter()
            .map(|input| kind.evaluate(self.radix, input))
            .collect::<Vec<_>>();
        let wire = self.domain.len();
        let output = (wire..wire + table[0].len()).collect::<Vec<_>>();
        for index in 0..output.len() {
            let mut domain = table.iter().map(|output| output[index]).collect::<Vec<_>>();
            domain.sort();
            domain.dedup();
            self.domain.push(domain);
        }
        self.gate.push(Gate {
            input,
            output: output.clone(),
            kind,
        });
        output
    }
}

pub fn multiply(radix: u8, width: usize, left: u64, right: u64) -> Result<String, Failure> {
    let mut circuit = Circuit::new(radix, width, left, right)?;
    let mut column = vec![Vec::new(); width * 2 + 1];
    for left in 0..width {
        for right in 0..width {
            let output = circuit.append(vec![left, width + right], Kind::Product);
            column[left + right].push(output[0]);
            if circuit.domain[output[1]] != [0] {
                column[left + right + 1].push(output[1]);
            }
        }
    }
    Ok(circuit.reduce(column))
}

pub fn add(radix: u8, width: usize, left: u64, right: u64) -> Result<String, Failure> {
    let circuit = Circuit::new(radix, width, left, right)?;
    let mut column = vec![Vec::new(); width + 2];
    for (index, column) in column.iter_mut().enumerate().take(width) {
        column.extend([index, width + index]);
    }
    Ok(circuit.reduce(column))
}

impl Circuit {
    fn new(radix: u8, width: usize, left: u64, right: u64) -> Result<Self, Failure> {
        let format = Format::operand(radix, width)?;
        format.check(left)?;
        format.check(right)?;
        Ok(Self {
            radix,
            domain: vec![(0..radix).collect(); width * 2],
            gate: Vec::new(),
            initial: [left, right]
                .into_iter()
                .enumerate()
                .flat_map(|(index, value)| {
                    (0..width).map(move |offset| {
                        (
                            index * width + offset,
                            (value / (radix as u64).pow(offset as u32) % radix as u64) as u8,
                        )
                    })
                })
                .collect(),
        })
    }

    fn constant(&mut self, value: u8) -> usize {
        let wire = self.domain.len();
        self.initial.push((wire, value));
        self.domain.push(vec![value]);
        wire
    }

    fn subtract(&mut self, left: &[usize], right: &[usize], zero: usize) -> (Vec<usize>, usize) {
        let mut borrow = zero;
        let mut result = Vec::new();
        for (&left, &right) in left.iter().zip(right) {
            let output = self.append(vec![left, right, borrow], Kind::Difference);
            result.push(output[0]);
            borrow = output[1];
        }
        (result, borrow)
    }

    fn select(&mut self, left: &[usize], right: &[usize], choice: usize) -> Vec<usize> {
        left.iter()
            .zip(right)
            .map(|(&left, &right)| self.append(vec![left, right, choice], Kind::Select)[0])
            .collect()
    }
}

pub fn subtract(radix: u8, width: usize, left: u64, right: u64) -> Result<String, Failure> {
    let mut circuit = Circuit::new(radix, width, left, right)?;
    let zero = circuit.constant(0);
    let left = (0..width).collect::<Vec<_>>();
    let right = (width..width * 2).collect::<Vec<_>>();
    let (forward, negative) = circuit.subtract(&left, &right, zero);
    let (reverse, _) = circuit.subtract(&right, &left, zero);
    let magnitude = circuit.select(&forward, &reverse, negative);
    let mut result = magnitude
        .into_iter()
        .enumerate()
        .map(|(index, wire)| (Address::at("Digit", index), wire))
        .collect::<Vec<_>>();
    result.push((Address::scalar("Negative"), negative));
    Ok(circuit.emit(result))
}

pub fn divide(radix: u8, width: usize, left: u64, right: u64) -> Result<String, Failure> {
    let mut circuit = Circuit::new(radix, width, left, right)?;
    let zero = circuit.constant(0);
    let divisor = (width..width * 2).chain([zero]).collect::<Vec<_>>();
    let mut remainder = vec![zero; width];
    let mut quotient = vec![zero; width];
    let mut nonzero = zero;
    for &wire in &divisor[..width] {
        nonzero = circuit.append(vec![nonzero, wire], Kind::Union)[0];
    }
    for index in (0..width).rev() {
        let shifted = std::iter::once(index).chain(remainder).collect::<Vec<_>>();
        let mut current = shifted;
        let mut count = zero;
        for _ in 0..radix - 1 {
            let (difference, borrow) = circuit.subtract(&current, &divisor, zero);
            current = circuit.select(&difference, &current, borrow);
            let accepted = circuit.append(vec![borrow], Kind::Invert)[0];
            count = if count == zero {
                accepted
            } else {
                circuit.append(vec![count, accepted], Kind::Sum)[0]
            };
        }
        remainder = current[..width].to_vec();
        quotient[index] = circuit.append(vec![count, nonzero], Kind::Product)[0];
    }

    let undefined = circuit.append(vec![nonzero], Kind::Invert)[0];
    let mut result = quotient
        .into_iter()
        .enumerate()
        .map(|(index, wire)| (Address::at("Quotient", index), wire))
        .collect::<Vec<_>>();
    result.extend(
        remainder
            .into_iter()
            .enumerate()
            .map(|(index, wire)| (Address::at("Remainder", index), wire)),
    );
    result.push((Address::scalar("Undefined"), undefined));
    Ok(circuit.emit(result))
}
