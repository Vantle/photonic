use photonic::lowering::parse;
use photonic::path::{Search, Summary};
use photonic::runtime::Limit;

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

pub struct Fixture {
    initial: Vec<String>,
    rule: Vec<String>,
    stage: usize,
}

impl Fixture {
    pub fn new() -> Self {
        Self {
            initial: vec!["Setup.0".into()],
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

    pub fn vector(&mut self, item: &[Vec<u64>], start: &str, label: &str, done: &str) {
        let holder = self.stage();
        let mut next = self.stage();
        self.rule
            .push(format!("[{start}] ({holder}.Empty) ({next})"));
        for digit in item.iter().rev() {
            let value = self.stage();
            let built = self.stage();
            self.natural(digit, &next, &value, &built);
            let wait = self.stage();
            self.initial.push(wait.clone());
            self.rule
                .push(format!("[Clean.{built}, {value}, {holder}] Insert"));
            let after = self.stage();
            self.rule
                .push(format!("[Stored, {wait}] ({holder}) (Release.{after})"));
            next = format!("Released.{after}");
        }
        self.rule
            .push(format!("[{next}, {holder}] ({label}) (Release.{done})"));
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

    pub fn inspect(&mut self, item: &[Vec<u64>], label: &str, done: &str) {
        let mut take = self.stage();
        self.rule.push(format!("[{label}] Take.{take}"));
        for digit in item {
            let rest = self.stage();
            let value = self.stage();
            let checked = self.stage();
            let next = self.stage();
            self.rule.push(format!(
                "[Taken.Item.{take}] (Forget.{rest}) (Release.{value})"
            ));
            self.number(digit, &format!("Released.{value}"), &checked);
            self.rule
                .push(format!("[Clean.{rest}, {checked}] Take.{next}"));
            take = next;
        }
        self.rule.push(format!("[Taken.End.Empty.{take}] {done}"));
    }

    pub fn source(&self) -> String {
        format!("{}\n{}", self.initial.join(", "), self.rule.join("\n"))
    }
}

pub fn trace(source: &str, target: &str, library: &[&str]) -> Summary {
    let program = crate::program(source, library);
    let target = photonic::source::Program {
        rule: program.rule.clone(),
        ..parse(target).unwrap()
    };
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
    search.summary()
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
