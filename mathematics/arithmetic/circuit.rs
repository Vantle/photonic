use crate::encoding::{digit, label};
use crate::failure::Failure;
use crate::format::Format;
use crate::gate::assignment;
use crate::gate::{Gate, Kind};

struct Circuit {
    wire: usize,
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
        let output = (self.wire..self.wire + table[0].len()).collect::<Vec<_>>();
        self.wire += output.len();
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

#[derive(Clone, Copy)]
pub enum Layout {
    Column,
    Balanced,
}

pub fn multiply(
    radix: u8,
    width: usize,
    left: u64,
    right: u64,
    layout: Layout,
) -> Result<String, Failure> {
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
    Ok(circuit.reduce(column, layout))
}

pub fn add(radix: u8, width: usize, left: u64, right: u64) -> Result<String, Failure> {
    let circuit = Circuit::new(radix, width, left, right)?;
    let mut column = vec![Vec::new(); width + 2];
    for (index, column) in column.iter_mut().enumerate().take(width) {
        column.extend([index, width + index]);
    }
    Ok(circuit.reduce(column, Layout::Column))
}

impl Circuit {
    fn reduce(mut self, mut column: Vec<Vec<usize>>, layout: Layout) -> String {
        let width = column.len() - 1;
        while matches!(layout, Layout::Balanced) && column.iter().any(|column| column.len() > 2) {
            let mut next = vec![Vec::new(); column.len()];
            for (index, column) in column.into_iter().enumerate() {
                for group in column.chunks(3) {
                    if group.len() < 3 {
                        next[index].extend_from_slice(group);
                        continue;
                    }
                    let output = self.append(group.to_vec(), Kind::Sum);
                    next[index].push(output[0]);
                    next[index + 1].push(output[1]);
                }
            }
            column = next;
        }
        let mut result = Vec::new();
        for index in 0..width {
            while column[index].len() > 1 {
                let count = column[index].len().min(3);
                let offset = column[index].len() - count;
                let input = column[index].split_off(offset);
                let output = self.append(input, Kind::Sum);
                column[index].push(output[0]);
                column[index + 1].push(output[1]);
            }
            result.push(column[index].pop());
        }
        let result = result
            .into_iter()
            .enumerate()
            .map(|(index, wire)| {
                let wire = wire.unwrap_or_else(|| self.constant(0));
                (format!("{}{index}", label(self.radix)), wire)
            })
            .collect();
        self.emit(result)
    }

    fn new(radix: u8, width: usize, left: u64, right: u64) -> Result<Self, Failure> {
        let format = Format::operand(radix, width)?;
        format.check(left)?;
        format.check(right)?;
        Ok(Self {
            radix,
            domain: vec![(0..radix).collect(); width * 2],
            wire: width * 2,
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
        let wire = self.wire;
        self.wire += 1;
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

    fn emit(self, result: Vec<(String, usize)>) -> String {
        let mut live = vec![false; self.wire];
        for (_, wire) in &result {
            live[*wire] = true;
        }
        let mut retained = vec![false; self.gate.len()];
        for (index, gate) in self.gate.iter().enumerate().rev() {
            if gate.output.iter().any(|&wire| live[wire]) {
                retained[index] = true;
                for &wire in &gate.input {
                    live[wire] = true;
                }
            }
        }
        let gate = self
            .gate
            .iter()
            .zip(retained)
            .filter_map(|(gate, live)| live.then_some(gate))
            .collect::<Vec<_>>();
        let mut port = vec![Vec::<String>::new(); self.wire];
        let mut input = Vec::new();
        let mut ordinal = 0;
        for gate in &gate {
            let mut selected = Vec::new();
            for &wire in &gate.input {
                let label = format!("Port{ordinal}");
                ordinal += 1;
                port[wire].push(label.clone());
                selected.push(label);
            }
            input.push(selected);
        }
        for (label, wire) in result {
            port[wire].push(label);
        }
        let initial = self
            .initial
            .iter()
            .map(|(wire, value)| format!("Input{wire}{}", digit(*value)))
            .collect::<Vec<_>>();
        let mut source = initial.join(".") + "\n\n";
        for &(wire, _) in &self.initial {
            for &value in &self.domain[wire] {
                let output = port[wire]
                    .iter()
                    .map(|label| format!("{label}{}", digit(value)))
                    .collect::<Vec<_>>();
                source.push_str(&format!(
                    "[Input{wire}{}] {}\n",
                    digit(value),
                    particle(output)
                ));
            }
        }
        for (gate, input) in gate.into_iter().zip(input) {
            let domain = gate
                .input
                .iter()
                .map(|&wire| self.domain[wire].clone())
                .collect::<Vec<_>>();
            for value in assignment(&domain) {
                let pattern = input
                    .iter()
                    .zip(&value)
                    .map(|(label, &value)| format!("{label}{}", digit(value)))
                    .collect::<Vec<_>>()
                    .join(".");
                let output = gate
                    .output
                    .iter()
                    .zip(gate.kind.evaluate(self.radix, &value))
                    .flat_map(|(&wire, value)| {
                        port[wire]
                            .iter()
                            .map(move |label| format!("{label}{}", digit(value)))
                    })
                    .collect();
                source.push_str(&format!("[{pattern}] {}\n", particle(output)));
            }
        }
        source
    }
}

fn particle(value: Vec<String>) -> String {
    if value.is_empty() {
        "()".into()
    } else {
        value.join(".")
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
        .map(|(index, wire)| (format!("{}{index}", label(radix)), wire))
        .collect::<Vec<_>>();
    result.push(("Negative".into(), negative));
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
        .map(|(index, wire)| (format!("Quotient{index}"), wire))
        .collect::<Vec<_>>();
    result.extend(
        remainder
            .into_iter()
            .enumerate()
            .map(|(index, wire)| (format!("Remainder{index}"), wire)),
    );
    result.push(("Undefined".into(), undefined));
    Ok(circuit.emit(result))
}
