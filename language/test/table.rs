use crate::runtime::{Limit, Runtime};
use std::sync::Arc;

fn snapshot(runtime: &Runtime) -> serde_json::Value {
    let mut value = serde_json::to_value(runtime.snapshot()).unwrap();
    value.as_object_mut().unwrap().remove("record");
    value.as_object_mut().unwrap().remove("peak");
    value
}

fn evaluate(bounded: bool, worker: usize) {
    let padding = (0..24)
        .map(|position| format!("Padding{position}"))
        .collect::<Vec<_>>()
        .join(".");
    let source =
        format!("A.B.C.D.E.F.G.H.{padding},X,Y [A.B.C.D.E.F.G.H,X] Left [A.B.C.D.E.F.G.H,Y] Right");
    let create = |capacity| {
        let mut runtime = Runtime::new(crate::lowering::parse(&source).unwrap());
        runtime.matching.preparation = Some(Arc::new(crate::selection::Store::new(capacity)));
        runtime
    };
    let mut actual = create(65_536);
    let mut expected = create(0);
    let mut limit = Limit {
        state: 32,
        record: 1_000_000,
        world: 8,
        cell: 64,
        frame: 16,
    };
    let executor = crate::executor::Executor::new(worker).unwrap();
    let mut cached = false;
    for step in 0..10_000 {
        let budget = if bounded {
            1
        } else {
            [0, 1, 2, 7, 31][step % 5]
        };
        if bounded {
            limit.record = expected.record() + 1;
        }
        actual.parallel(&executor, budget, Some(limit));
        expected.run(budget, Some(limit));
        assert_eq!(snapshot(&actual), snapshot(&expected));
        cached |= actual.matching.preparation.as_ref().unwrap().retained() > 0;
        if actual.closed() {
            break;
        }
        if !bounded && step % 11 == 0 {
            actual.matching.evict();
        }
    }
    assert!(actual.closed());
    assert!(cached);
    assert_eq!(
        actual.snapshot().event.len(),
        expected.snapshot().event.len()
    );
}

#[test]
fn evidence() {
    evaluate(false, 1);
}

#[test]
fn pressure() {
    evaluate(true, 1);
}

#[test]
fn parallel() {
    evaluate(false, 4);
    evaluate(true, 4);
}
