use super::support::{configuration, input};
use crate::checkpoint::{Failure, load, save};
use crate::model::Model;
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
    assert_eq!(restored.parameter, model.parameter);
    assert_eq!(restored.configuration(), model.configuration());
    assert_eq!(state.step, 17);
    assert_eq!(state.moment, optimizer.moment);
    let probe = input(&mut generator, &configuration);
    assert_eq!(model.infer(&[&probe]), restored.infer(&[&probe]));
    std::fs::write(&path, b"{").unwrap();
    assert!(matches!(load(&path), Err(Failure::Header(_))));
    std::fs::remove_dir_all(&directory).unwrap();
}
