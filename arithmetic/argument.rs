use clap::Parser;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Operation {
    Add,
    Multiply,
    Subtract,
    Divide,
}

impl Operation {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Multiply => "×",
            Self::Subtract => "−",
            Self::Divide => "÷",
        }
    }
}

#[derive(Parser)]
#[command(about = "Prove a proposed fixed-width arithmetic result using ordinary Photonic rules")]
pub struct Argument {
    #[arg(long, value_enum, default_value_t = Operation::Multiply)]
    pub operation: Operation,
    #[arg(
        long,
        help = "Operand ternary digit count, inferred when omitted (maximum 20)"
    )]
    pub width: Option<usize>,
    #[arg(long, value_parser = arithmetic::numeral::unsigned)]
    pub left: u64,
    #[arg(long, value_parser = arithmetic::numeral::unsigned)]
    pub right: u64,
    #[arg(long, allow_hyphen_values = true, value_parser = arithmetic::numeral::signed)]
    pub expected: i128,
    #[arg(long, default_value_t = 0, value_parser = arithmetic::numeral::unsigned)]
    pub remainder: u64,
    #[arg(long)]
    pub undefined: bool,
    #[arg(long)]
    pub directory: Option<PathBuf>,
    #[arg(long, default_value_t = 20_000_000)]
    pub work: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum Failure {
    #[error(transparent)]
    Representation(#[from] arithmetic::failure::Failure),
    #[error("width must be 1 through 20, and both operands must fit")]
    Width,
    #[error("the proposed result must fit the output width")]
    Capacity,
    #[error("only subtraction accepts a negative proposed result")]
    Sign,
    #[error("the proposed remainder must fit the input width")]
    Remainder,
    #[error("remainder and undefined apply only to division")]
    Operation,
}

impl Argument {
    pub fn width(&self) -> Result<usize, Failure> {
        let width = self.width.unwrap_or_else(|| {
            let mut value = self.left.max(self.right);
            let mut width = 1;
            while value >= 3 {
                value /= 3;
                width += 1;
            }
            width
        });
        if !(1..=20).contains(&width)
            || self.left as u128 >= 3u128.pow(width as u32)
            || self.right as u128 >= 3u128.pow(width as u32)
        {
            return Err(Failure::Width);
        }
        Ok(width)
    }

    pub fn prepare(&self) -> Result<(String, String), Failure> {
        let width = self.width()?;
        let capacity = match self.operation {
            Operation::Add => width + 1,
            Operation::Multiply => width * 2,
            _ => width,
        };
        if self.expected.unsigned_abs() >= 3u128.pow(capacity as u32) {
            return Err(Failure::Capacity);
        }
        if self.expected < 0 && !matches!(self.operation, Operation::Subtract) {
            return Err(Failure::Sign);
        }
        if self.remainder as u128 >= 3u128.pow(width as u32) {
            return Err(Failure::Remainder);
        }
        if (self.remainder != 0 || self.undefined) && !matches!(self.operation, Operation::Divide) {
            return Err(Failure::Operation);
        }
        Ok(match self.operation {
            Operation::Add => (
                arithmetic::circuit::add(3, width, self.left, self.right)?,
                arithmetic::encoding::unsigned(3, width + 1, self.expected as u64)?,
            ),
            Operation::Multiply => (
                arithmetic::circuit::multiply(
                    3,
                    width,
                    self.left,
                    self.right,
                    arithmetic::circuit::Layout::Column,
                )?,
                arithmetic::encoding::unsigned(3, width * 2, self.expected as u64)?,
            ),
            Operation::Subtract => (
                arithmetic::circuit::subtract(3, width, self.left, self.right)?,
                arithmetic::encoding::difference(3, width, self.expected)?,
            ),
            Operation::Divide => (
                arithmetic::circuit::divide(3, width, self.left, self.right)?,
                arithmetic::encoding::quotient(
                    3,
                    width,
                    self.expected as u64,
                    self.remainder,
                    self.undefined,
                )?,
            ),
        })
    }
}
