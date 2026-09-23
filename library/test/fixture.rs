use photonic::path::Search;
use photonic::prism::Outcome;
use photonic::runtime::Limit;
use std::fmt::Display;

pub fn digit(mut value: u64) -> Vec<u64> {
    let mut digit = Vec::new();
    while value > 0 {
        digit.push(value % 3);
        value /= 3;
    }
    digit
}

pub fn value(digit: &[u64]) -> u64 {
    digit.iter().rev().fold(0, |value, digit| value * 3 + digit)
}

#[derive(Clone, Debug)]
pub enum Value {
    Natural(Vec<u64>),
    Vector(Vec<Self>),
}

impl Value {
    pub fn number(value: u64) -> Self {
        Self::Natural(digit(value))
    }
}

pub struct Fixture {
    rule: Vec<String>,
    stage: usize,
}

impl Fixture {
    pub fn new() -> Self {
        Self {
            rule: Vec::new(),
            stage: 0,
        }
    }

    pub fn start(&self) -> &'static str {
        "Setup.0"
    }

    pub fn stage(&mut self) -> String {
        self.stage += 1;
        format!("Setup.{}", self.stage)
    }

    pub fn rule(&mut self, rule: String) {
        self.rule.push(rule);
    }

    pub fn build(&mut self, value: &Value, start: &str, label: &str, done: &str) {
        match value {
            Value::Natural(digit) => self.natural(digit, start, label, done),
            Value::Vector(item) => self.vector(item, start, label, done),
        }
    }

    pub fn natural(&mut self, digit: &[u64], start: &str, label: &str, done: &str) {
        let Some((last, rest)) = digit.split_last() else {
            self.rule
                .push(format!("[{start}] ({label}.Zero) (Clean.{done})"));
            return;
        };
        let mut stage = self.stage();
        self.rule
            .push(format!("[{start}] (Push.([Digit] {last}).Zero) ({stage})"));
        for digit in rest.iter().rev() {
            let next = self.stage();
            self.rule.push(format!(
                "[Built, {stage}] (Push.([Digit] {digit})) (Forget.{next})"
            ));
            stage = format!("Clean.{next}");
        }
        self.rule
            .push(format!("[Built, {stage}] ({label}) (Forget.{done})"));
    }

    pub fn vector(&mut self, item: &[Value], start: &str, label: &str, done: &str) {
        let holder = self.stage();
        let mut next = self.stage();
        self.rule
            .push(format!("[{start}] ({holder}.Empty) ({next})"));
        for value in item.iter().rev() {
            let piece = self.stage();
            let built = self.stage();
            self.build(value, &next, &piece, &built);
            let wait = self.stage();
            let go = self.stage();
            let link = self.stage();
            self.rule.push(format!("[Clean.{built}] ({wait}) ({go})"));
            self.rule.push(format!("[{go}, {holder}] Link.{link}"));
            self.rule.push(format!("[Linked.{link}, {piece}] Insert"));
            let after = self.stage();
            self.rule
                .push(format!("[Stored, {wait}] ({holder}) (Forget.{after})"));
            next = format!("Clean.{after}");
        }
        self.rule
            .push(format!("[{next}, {holder}] ({label}) (Forget.{done})"));
    }

    pub fn number(&mut self, digit: &[u64], label: &str, done: &str) {
        let mut read = self.stage();
        self.rule.push(format!("[{label}] Read.{read}"));
        for digit in digit {
            let next = self.stage();
            self.rule
                .push(format!("[Yield.([Digit] {digit}).{read}] Read.{next}"));
            read = next;
        }
        self.rule.push(format!("[Yield.End.{read}.Zero] {done}"));
    }

    pub fn inspect(&mut self, value: &Value, label: &str, done: &str) {
        match value {
            Value::Natural(digit) => self.number(digit, label, done),
            Value::Vector(item) => self.walk(item, label, done),
        }
    }

    fn walk(&mut self, item: &[Value], label: &str, done: &str) {
        let mut current = label.to_string();
        for value in item {
            let take = self.stage();
            let piece = self.stage();
            let checked = self.stage();
            let hold = self.stage();
            self.rule.push(format!("[{current}] Take.{take}"));
            self.rule.push(format!("[Taken.Item.{take}] {piece}"));
            self.inspect(value, &piece, &checked);
            self.rule
                .push(format!("[Taken.Rest.{take}, {checked}] {hold}"));
            current = hold;
        }
        let take = self.stage();
        self.rule.push(format!("[{current}] Take.{take}"));
        self.rule.push(format!("[Taken.End.Empty.{take}] {done}"));
    }

    pub fn source(&self) -> String {
        format!("{}\n{}", self.start(), self.rule.join("\n"))
    }
}

pub fn reach(source: &str, target: &str, library: &[&str], context: impl Display) {
    let program = crate::program(source, library);
    let target = crate::target(&program, target);
    let mut search = Search::new(program, target);
    search.run(
        200_000_000,
        Limit {
            state: 65536,
            record: 100_000_000,
            cell: 16384,
            world: 1024,
            frame: 2048,
        },
    );
    assert_eq!(search.summary().outcome, Outcome::Reached, "{context}");
}

pub struct Random(u64);

impl Random {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn below(&mut self, bound: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % bound
    }
}
