use super::Circuit;
use crate::address::Address;
use crate::gate::assignment;

impl Circuit {
    pub(super) fn emit(self, result: Vec<(Address, usize)>) -> String {
        let mut live = vec![false; self.domain.len()];
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
        let mut port = vec![Vec::<Address>::new(); self.domain.len()];
        let mut input = Vec::new();
        let mut ordinal = 0;
        for gate in &gate {
            let mut selected = Vec::new();
            for &wire in &gate.input {
                let label = Address::at("Port", ordinal);
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
            .map(|(wire, value)| Address::at("Input", *wire).rule(*value))
            .collect::<Vec<_>>();
        let mut rule = Vec::new();
        for &(wire, _) in &self.initial {
            for &value in &self.domain[wire] {
                let output = port[wire]
                    .iter()
                    .map(|label| label.rule(value))
                    .collect::<Vec<_>>();
                rule.push(format!(
                    "[{}] {}",
                    Address::at("Input", wire).rule(value),
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
                    .map(|(label, &value)| label.rule(value))
                    .collect();
                let output = gate
                    .output
                    .iter()
                    .zip(gate.kind.evaluate(self.radix, &value))
                    .flat_map(|(&wire, value)| {
                        port[wire].iter().map(move |label| label.rule(value))
                    })
                    .collect();
                rule.push(format!("[{}] {}", particle(pattern), particle(output)));
            }
        }
        let rule = rule.join(",\n");
        match &initial[..] {
            [] => rule + "\n",
            [field] => format!("().({field}),\n\n{rule}\n"),
            _ => format!("{},\n\n{rule}\n", join(&initial)),
        }
    }
}

fn particle(value: Vec<String>) -> String {
    match &value[..] {
        [] => "()".into(),
        [rule] => rule.clone(),
        _ => join(&value),
    }
}

fn join(value: &[String]) -> String {
    value
        .iter()
        .map(|rule| format!("({rule})"))
        .collect::<Vec<_>>()
        .join(".")
}
