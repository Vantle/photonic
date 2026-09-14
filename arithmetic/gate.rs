#[cfg(test)]
#[path = "gate/test.rs"]
mod test;

#[derive(Clone, Copy)]
pub enum Kind {
    Product,
    Sum,
    Difference,
    Select,
    Invert,
    Union,
}

impl Kind {
    pub fn evaluate(self, radix: u8, input: &[u8]) -> Vec<u8> {
        let left = input[0];
        let right = input.get(1).copied().unwrap_or(0);
        let third = input.get(2).copied().unwrap_or(0);
        match self {
            Self::Product => vec![left * right % radix, left * right / radix],
            Self::Sum => {
                let sum: u8 = input.iter().sum();
                vec![sum % radix, sum / radix]
            }
            Self::Difference => vec![
                (left + radix - right - third) % radix,
                u8::from(left < right + third),
            ],
            Self::Select => vec![if third == 0 { left } else { right }],
            Self::Invert => vec![u8::from(left == 0)],
            Self::Union => vec![u8::from(left != 0 || right != 0)],
        }
    }
}

pub struct Gate {
    pub input: Vec<usize>,
    pub output: Vec<usize>,
    pub kind: Kind,
}

pub fn assignment(domain: &[Vec<u8>]) -> Vec<Vec<u8>> {
    domain.iter().fold(vec![Vec::new()], |prefix, domain| {
        prefix
            .into_iter()
            .flat_map(|prefix| {
                domain.iter().map(move |&value| {
                    let mut result = prefix.clone();
                    result.push(value);
                    result
                })
            })
            .collect()
    })
}
