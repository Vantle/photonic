use super::support::{configuration, input};
use crate::configuration::Configuration;
use crate::grow::{Failure, grow};
use crate::model::Model;
use random::Generator;

#[test]
fn preserve() {
    let mut generator = Generator::new(61);
    let before = configuration();
    let mut model = Model::new(before.clone(), &mut generator);
    for value in model.edit() {
        *value += generator.normal() as f32 * 0.3;
    }
    let after = Configuration {
        width: 16,
        depth: 3,
        head: 4,
        hidden: 20,
        unary: before.unary + 1,
        binary: before.binary + 1,
        ..before.clone()
    };
    let grown = grow(&model, after, &mut generator).unwrap();
    assert!(grown.size() > 2 * model.size());
    let input = (0..6)
        .map(|_| input(&mut generator, &before))
        .collect::<Vec<_>>();
    let reference = input.iter().collect::<Vec<_>>();
    for (expected, actual) in model.infer(&reference).iter().zip(grown.infer(&reference)) {
        assert!((expected.value - actual.value).abs() < 1e-4);
        for (left, right) in expected.logit.iter().zip(&actual.logit) {
            assert!((left - right).abs() < 1e-4, "{left} {right}");
        }
    }
    let project = grown.block()[0].attention.project.weight;
    let row = |index: usize| {
        &project.read(grown.parameter())[index * project.column..(index + 1) * project.column]
    };
    assert_ne!(row(0), row(before.width));
}

#[test]
fn widen() {
    let mut generator = Generator::new(62);
    let before = configuration();
    let mut model = Model::new(before.clone(), &mut generator);
    for value in model.edit() {
        *value += generator.normal() as f32 * 0.3;
    }
    let mut field = before.field.clone();
    field[0] += 3;
    field.extend([5, 7]);
    let after = Configuration { field, ..before };
    let grown = grow(&model, after.clone(), &mut generator).unwrap();
    let input = (0..6)
        .map(|_| input(&mut generator, &before))
        .collect::<Vec<_>>();
    let wide = input
        .iter()
        .map(|entry| {
            let mut entry = entry.clone();
            entry.feature = entry
                .feature
                .chunks(before.field.len())
                .flat_map(|token| {
                    token.iter().copied().chain(std::iter::repeat_n(
                        0,
                        after.field.len() - before.field.len(),
                    ))
                })
                .collect();
            entry
        })
        .collect::<Vec<_>>();
    let narrow = input.iter().collect::<Vec<_>>();
    let widened = wide.iter().collect::<Vec<_>>();
    for (expected, actual) in model.infer(&narrow).iter().zip(grown.infer(&widened)) {
        assert!((expected.value - actual.value).abs() < 1e-4);
        for (left, right) in expected.logit.iter().zip(&actual.logit) {
            assert!((left - right).abs() < 1e-4, "{left} {right}");
        }
    }
}

#[test]
fn refuse() {
    let before = configuration();
    let model = Model::new(before.clone(), &mut Generator::new(1));
    let case = [
        (
            Configuration {
                width: 12,
                head: 3,
                ..before.clone()
            },
            Failure::Width { from: 8, to: 12 },
        ),
        (
            Configuration {
                width: 16,
                ..before.clone()
            },
            Failure::Head { dimension: 4 },
        ),
        (
            Configuration {
                depth: 1,
                ..before.clone()
            },
            Failure::Shrink,
        ),
        (
            Configuration {
                unary: 2,
                ..before.clone()
            },
            Failure::Shrink,
        ),
        (
            Configuration {
                field: before.field[1..].to_vec(),
                ..before.clone()
            },
            Failure::Interface,
        ),
        (
            Configuration {
                field: before.field.iter().map(|size| size - 1).collect(),
                ..before.clone()
            },
            Failure::Shrink,
        ),
        (Configuration { key: 8, ..before }, Failure::Interface),
    ];
    for (configuration, failure) in case {
        assert_eq!(
            grow(&model, configuration, &mut Generator::new(2)).unwrap_err(),
            failure
        );
    }
}
