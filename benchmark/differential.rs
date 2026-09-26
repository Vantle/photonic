use serde_json::Value;
use std::collections::BTreeSet;

fn configuration(value: &Value) -> Vec<Vec<String>> {
    assert_eq!(value["frame"].as_array().unwrap().len(), 1);
    assert!(value["frame"][0]["held"].as_array().unwrap().is_empty());
    let mut identity = BTreeSet::new();
    let mut world = value["world"]
        .as_array()
        .unwrap()
        .iter()
        .map(|world| {
            assert_eq!(world["frame"], 0);
            let mut particle = world["particle"]
                .as_array()
                .unwrap()
                .iter()
                .map(|token| {
                    assert!(identity.insert(token["id"].to_string()));
                    assert!(token.get("capture").is_none());
                    token["display"]
                        .as_str()
                        .or_else(|| token["atom"].as_str())
                        .unwrap()
                        .to_owned()
                })
                .collect::<Vec<_>>();
            particle.sort();
            particle
        })
        .collect::<Vec<_>>();
    world.sort();
    world
}

fn observation(value: impl serde::Serialize) -> Value {
    let value = serde_json::to_value(value).unwrap();
    let state = value["state"].as_array().unwrap();
    let configuration = state.iter().map(configuration).collect::<Vec<_>>();
    let node = state
        .iter()
        .zip(&configuration)
        .map(|(state, configuration)| (configuration, state["status"].as_str().unwrap()))
        .collect::<BTreeSet<_>>();
    let event = value["event"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| {
            (
                &configuration[event["source"].as_u64().unwrap() as usize],
                &configuration[event["target"].as_u64().unwrap() as usize],
                event["rule"].as_str().unwrap_or_else(|| {
                    value["definition"][event["rule"].as_u64().unwrap() as usize]["name"]
                        .as_str()
                        .unwrap()
                }),
                event["status"].as_str().unwrap(),
            )
        })
        .collect::<BTreeSet<_>>();
    serde_json::json!({"state": node, "event": event, "closed": value["closed"]})
}

fn compare(source: &str, target: &str) {
    let program = frontend::lowering::parse(source).unwrap();
    let limit = photonic::runtime::Limit {
        configuration: 4096,
        record: 2_000_000,
        coherence: 8,
        occurrence: 128,
        scope: 8,
    };
    let reference = reference::runtime::Limit {
        state: limit.configuration,
        record: limit.record,
        world: limit.coherence,
        cell: limit.occurrence,
        frame: limit.scope,
    };
    let mut current = photonic::runtime::Runtime::new(&program);
    let mut previous =
        reference::runtime::Runtime::new(reference::lowering::parse(source).unwrap());
    current.run(1, photonic::runtime::Limit { record: 1, ..limit });
    for budget in [0, 1, 2, 7, 31, 127, 511, 2048] {
        current.run(budget, limit);
    }
    current.run(1_000_000, limit);
    previous.run(1_000_000, Some(reference));
    assert!(current.closed(), "{source}");
    assert!(previous.closed(), "{source}");
    assert_eq!(
        observation(current.snapshot()),
        observation(previous.snapshot()),
        "{source}"
    );
    let mut target = frontend::lowering::parse(target).unwrap();
    target.preserve(&program);
    let mut current = photonic::path::Search::new(program, Some(target.clone()));
    let mut previous = reference::path::Search::new(
        reference::lowering::parse(source).unwrap(),
        serde_json::from_value(serde_json::json!({"initial": target.initial})).unwrap(),
    )
    .unwrap();
    for budget in [0, 1, 2, 7, 31, 127, 511, 2048] {
        current.run(budget, limit);
    }
    current.run(1_000_000, limit);
    previous.run(1_000_000, reference);
    assert_eq!(
        serde_json::to_value(current.summary().outcome).unwrap(),
        serde_json::to_value(previous.summary().outcome).unwrap(),
        "{source}"
    );
    assert_eq!(
        configuration(&serde_json::to_value(current.current()).unwrap()),
        configuration(&serde_json::to_value(previous.current()).unwrap()),
        "{source}"
    );
}

fn run() -> usize {
    let mut count = 0;
    for width in 1..=3 {
        for depth in 1..=4 {
            for quantity in 1..=3 {
                for extra in 0..=2 {
                    for noise in 0..=4 {
                        let particle = |stage| {
                            std::iter::repeat_n(format!("Stage{stage}"), quantity)
                                .chain(std::iter::repeat_n("Extra".to_owned(), extra))
                                .collect::<Vec<_>>()
                                .join(".")
                        };
                        let mut source = vec![particle(0); width].join(",");
                        for stage in 0..depth {
                            source.push_str(&format!(
                                ", [{}] {}",
                                vec![format!("Stage{stage}"); quantity].join("."),
                                vec![format!("Stage{}", stage + 1); quantity].join(".")
                            ));
                        }
                        for index in 0..noise {
                            source.push_str(&format!(", [Absent{index}] Never{index}"));
                        }
                        let target = vec![particle(depth); width].join(",");
                        compare(&source, &target);
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

#[test]
fn regression() {
    assert_eq!(run(), 540);
}
