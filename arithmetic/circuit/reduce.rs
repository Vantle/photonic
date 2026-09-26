use super::Circuit;
use crate::address::Address;
use crate::gate::Kind;

impl Circuit {
    pub(super) fn reduce(mut self, mut column: Vec<Vec<usize>>) -> String {
        let width = column.len() - 1;
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
                (Address::at("Digit", index), wire)
            })
            .collect();
        self.emit(result)
    }
}
