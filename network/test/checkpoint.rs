use super::support::{configuration, input};
use crate::checkpoint::{Failure, load, save};
use crate::configuration::Configuration;
use crate::model::{Mismatch, Model};
use crate::optimizer::{Optimizer, Setting};
use random::Generator;

#[test]
fn roundtrip() {
    let mut generator = Generator::new(61);
    let configuration = configuration();
    let model = Model::new(configuration.clone(), &mut generator);
    let mut optimizer = Optimizer::new(model.size(), Setting::default());
    optimizer.step = 17;
    optimizer.moment[3] = 0.25;
    let directory = std::env::temp_dir().join(format!("network-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("model.checkpoint");
    save(&path, &model, &optimizer).unwrap();
    let (restored, state) = load(&path).unwrap();
    assert_eq!(restored.parameter(), model.parameter());
    assert_eq!(restored.configuration(), model.configuration());
    assert_eq!(state.step, 17);
    assert_eq!(state.moment, optimizer.moment);
    let probe = input(&mut generator, &configuration);
    assert_eq!(model.infer(&[&probe]), restored.infer(&[&probe]));
    std::fs::write(&path, b"{").unwrap();
    assert!(matches!(load(&path), Err(Failure::Header(_))));
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn length() {
    let base = configuration();
    for configuration in [
        base.clone(),
        Configuration {
            width: 16,
            head: 4,
            depth: 3,
            hidden: 20,
            judge: 0,
            ..base.clone()
        },
        Configuration {
            binary: 0,
            unary: 1,
            ..base
        },
    ] {
        let model = Model::new(configuration.clone(), &mut Generator::new(3));
        assert_eq!(Model::length(&configuration), Some(model.size()));
    }
    let mut model = Model::new(configuration(), &mut Generator::new(4));
    let size = model.size();
    assert_eq!(
        model.load(vec![0.0; size - 1]),
        Err(Mismatch {
            expected: size,
            found: size - 1
        })
    );
    model.load(vec![0.5; size]).unwrap();
    assert!(model.parameter().iter().all(|&value| value == 0.5));
}

#[test]
fn malformed() {
    let directory = std::env::temp_dir().join(format!("network-malformed-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("model.checkpoint");
    let header = |configuration: Configuration| {
        let line = serde_json::json!({
            "configuration": configuration,
            "setting": Setting::default(),
            "step": 0,
            "parameter": 0,
        });
        std::fs::write(&path, format!("{line}\n")).unwrap();
        load(&path)
    };
    let valid = configuration();
    assert!(matches!(
        header(Configuration {
            field: Vec::new(),
            ..valid
        }),
        Err(Failure::Field)
    ));
    assert!(matches!(
        header(Configuration {
            width: 0,
            ..valid.clone()
        }),
        Err(Failure::Width)
    ));
    assert!(matches!(
        header(Configuration {
            width: 1,
            head: 1,
            depth: 1_000_000_000_000,
            ..valid.clone()
        }),
        Err(Failure::Shape { found: 0, .. })
    ));
    assert!(matches!(
        header(Configuration {
            width: 1 << 62,
            head: 1 << 62,
            ..valid
        }),
        Err(Failure::Overflow)
    ));
    std::fs::remove_dir_all(&directory).unwrap();
}
