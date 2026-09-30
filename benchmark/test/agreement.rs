use super::{Outcome, Reduction};

fn outcome() -> Outcome {
    Outcome {
        name: "sample".to_owned(),
        group: "test".to_owned(),
        interpreter: None,
        laser: None,
        verdict: Some("agree".to_owned()),
        plain: Some("within".to_owned()),
        reduction: Some(Reduction {
            verdict: "preserves".to_owned(),
            plain: 3,
            reduced: 2,
        }),
        net: Some("mirrors".to_owned()),
        metal: None,
        state: 3,
        event: 2,
        inferred: 0,
    }
}

#[test]
fn agreement() {
    assert!(outcome().agrees());
    let open = Outcome {
        verdict: None,
        plain: None,
        reduction: None,
        net: None,
        ..outcome()
    };
    assert!(open.agrees());
    let disagreement = [
        Outcome {
            verdict: Some("Event".to_owned()),
            ..outcome()
        },
        Outcome {
            plain: Some("Missing".to_owned()),
            ..outcome()
        },
        Outcome {
            net: Some("Configuration".to_owned()),
            ..outcome()
        },
        Outcome {
            metal: Some("Cycle".to_owned()),
            ..outcome()
        },
        Outcome {
            reduction: Some(Reduction {
                verdict: "End".to_owned(),
                plain: 3,
                reduced: 2,
            }),
            ..outcome()
        },
    ];
    for outcome in disagreement {
        assert!(!outcome.agrees(), "{}", outcome.name);
    }
}
