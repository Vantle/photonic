use code::atom::Atom;
use code::configuration::Configuration;
use code::output::Output;
use code::particle::Particle;
use code::rule::Rule;
use code::value::Value;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const A: u16 = 0;
pub const B: u16 = 1;
pub const C: u16 = 2;
pub const D: u16 = 3;
pub const X: u16 = 5;
pub const Y: u16 = 6;

pub fn particle(atom: &[u16]) -> Particle {
    Particle::atom(&atom.iter().map(|&value| Atom(value)).collect::<Vec<_>>())
}

pub fn configuration(coherence: &[&[u16]]) -> Configuration {
    Configuration::from(
        coherence
            .iter()
            .map(|atom| particle(atom))
            .collect::<Vec<_>>(),
    )
}

pub fn rule(input: &[&[u16]], output: &[&[u16]]) -> Rule {
    Rule::new(
        input.iter().map(|atom| particle(atom)).collect(),
        output
            .iter()
            .map(|atom| Output::Particle(particle(atom)))
            .collect(),
    )
}

pub fn nested(rule: Rule) -> Value {
    Value::Rule(Box::new(rule))
}

static NEXT: AtomicUsize = AtomicUsize::new(0);

pub struct Directory {
    pub path: PathBuf,
}

impl Directory {
    pub fn new(name: &str) -> Self {
        let base = std::env::var_os("TEST_TMPDIR").map_or_else(std::env::temp_dir, PathBuf::from);
        let path = base.join(format!(
            "learning-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
