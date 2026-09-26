use super::support::{configuration, sample};
use crate::loss::Weight;
use crate::model::Model;
use crate::optimizer::{Optimizer, Setting};
use random::Generator;

#[test]
fn overfit() {
    let mut generator = Generator::new(51);
    let configuration = configuration();
    let mut model = Model::new(configuration.clone(), &mut generator);
    let sample = (0..6)
        .map(|_| sample(&mut generator, &configuration))
        .collect::<Vec<_>>();
    let reference = sample.iter().collect::<Vec<_>>();
    let mut optimizer = Optimizer::new(
        model.size(),
        Setting {
            rate: 1e-2,
            ..Setting::default()
        },
    );
    let weight = Weight {
        value: 1.0,
        judge: 1.0,
        scale: 1.0 / sample.len() as f32,
    };
    let mut first = None;
    let mut last = 0.0;
    for _ in 0..300 {
        let mut gradient = vec![0.0; model.size()];
        let loss = model.gradient(&reference, &mut gradient, weight);
        let total = loss.policy + loss.value;
        first.get_or_insert(total);
        last = total;
        let decay = model.decay().to_vec();
        let rate = optimizer.setting.rate;
        optimizer.update(model.edit(), &gradient, &decay, rate);
    }
    let first = first.unwrap();
    assert!(last < 0.8 * first, "{first} -> {last}");
}
