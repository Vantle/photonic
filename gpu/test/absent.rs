use crate::engine::Engine;
use crate::failure::Failure;
use network::configuration::Configuration;
use network::model::Model;
use random::Generator;

#[test]
fn unavailable() {
    let configuration = Configuration {
        field: vec![3],
        width: 4,
        depth: 1,
        head: 1,
        hidden: 4,
        unary: 1,
        binary: 1,
        key: 4,
        judge: 2,
    };
    let model = Model::new(configuration, &mut Generator::new(1));
    assert!(matches!(Engine::new(&model), Err(Failure::Unavailable(_))));
}
