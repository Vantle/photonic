use super::support::{configuration, input};
use crate::model::Model;
use random::Generator;

#[test]
fn independent() {
    let mut generator = Generator::new(41);
    let configuration = configuration();
    let model = Model::new(configuration.clone(), &mut generator);
    let input = (0..5)
        .map(|_| input(&mut generator, &configuration))
        .collect::<Vec<_>>();
    let joint = model.infer(&input.iter().collect::<Vec<_>>());
    for (input, joint) in input.iter().zip(&joint) {
        let alone = &model.infer(&[input])[0];
        assert_eq!(alone.logit.len(), input.pointer.len());
        assert!((alone.value - joint.value).abs() < 1e-5);
        for (left, right) in alone.logit.iter().zip(&joint.logit) {
            assert!((left - right).abs() < 1e-5);
        }
    }
    assert!(model.infer(&[]).is_empty());
}
