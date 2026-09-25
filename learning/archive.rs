use crate::objective::{Evaluation, Size, TOLERANCE};
use crate::task::Goal;
use code::program::Program;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub fn moment() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub program: Program,
    pub cost: f64,
    pub correctness: f64,
    pub time: f64,
    pub work: f64,
    pub span: f64,
    pub size: usize,
    pub verified: bool,
    pub general: bool,
    pub proof: Option<Goal>,
    pub moment: u64,
}

impl Record {
    pub fn new(program: Program, evaluation: &Evaluation, general: bool, moment: u64) -> Self {
        Self {
            program,
            cost: evaluation.cost,
            correctness: evaluation.correctness,
            time: evaluation.time,
            work: evaluation.work,
            span: evaluation.span,
            size: Size::total(&evaluation.size),
            verified: evaluation.verified,
            general,
            proof: None,
            moment,
        }
    }

    pub fn prove(self, goal: Goal) -> Self {
        Self {
            proof: Some(goal),
            ..self
        }
    }

    pub fn revise(self, evaluation: &Evaluation, general: bool, goal: Goal) -> Self {
        let proof = self
            .proof
            .filter(|proven| *proven == goal && (evaluation.cost - self.cost).abs() <= TOLERANCE);
        Self {
            proof,
            ..Self::new(self.program, evaluation, general, self.moment)
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub best: Option<Record>,
    pub partial: Option<Record>,
    pub baseline: f64,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Archive {
    entry: BTreeMap<String, Entry>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Improvement {
    pub task: String,
    pub before: Option<f64>,
    pub after: f64,
    pub baseline: f64,
    pub record: Record,
}

impl Archive {
    pub fn entry(&self, task: &str) -> Option<&Entry> {
        self.entry.get(task)
    }

    pub fn best(&self, task: &str) -> f64 {
        self.entry
            .get(task)
            .and_then(|entry| entry.best.as_ref())
            .map_or(f64::INFINITY, |record| record.cost)
    }

    pub fn partial(&self, task: &str) -> f64 {
        self.entry
            .get(task)
            .and_then(|entry| entry.partial.as_ref())
            .map_or(0.0, |record| record.correctness)
    }

    pub fn register(&mut self, task: &str, baseline: f64) -> Entry {
        self.entry
            .insert(
                task.to_owned(),
                Entry {
                    baseline,
                    ..Entry::default()
                },
            )
            .unwrap_or_default()
    }

    pub fn offer(&mut self, task: &str, record: Record) -> Option<Improvement> {
        let entry = self.entry.entry(task.to_owned()).or_default();
        if record.correctness >= 1.0 && !record.general {
            return None;
        }
        if record.correctness < 1.0 {
            if entry
                .partial
                .as_ref()
                .is_none_or(|known| record.correctness > known.correctness)
            {
                entry.partial = Some(record);
            }
            return None;
        }
        if let Some(known) = entry.best.as_mut()
            && record.proof.is_some()
            && known.proof.is_none()
            && (record.cost - known.cost).abs() <= TOLERANCE
        {
            known.proof = record.proof;
            return None;
        }
        let before = entry.best.as_ref().map(|known| known.cost);
        if before.is_some_and(|known| record.cost >= known - TOLERANCE) {
            return None;
        }
        entry.best = Some(record.clone());
        Some(Improvement {
            task: task.to_owned(),
            before,
            after: record.cost,
            baseline: entry.baseline,
            record,
        })
    }

    pub fn task(&self) -> impl Iterator<Item = (&String, &Entry)> {
        self.entry.iter()
    }
}
