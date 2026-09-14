use super::{Circuit, Layout};
use crate::address::Address;
use crate::gate::Kind;

impl Circuit {
    pub(super) fn reduce(mut self, mut column: Vec<Vec<usize>>, layout: Layout) -> String {
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
                (Address::at("Digit", index), wire)
            })
            .collect();
        self.emit(result)
    }
}
