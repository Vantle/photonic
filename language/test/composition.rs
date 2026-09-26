use crate::flow::Flow;
use crate::runtime::{Limit, Runtime};
use std::sync::Arc;

#[test]
fn identity() {
    for source in [
        "A, [A] B, [B] C",
        "A,B, [A,B] (C, D), [C,D] E",
        "A, [A] B, [B] A",
        "A, [A] (B, C), [B] D, [C] E",
        include_str!("../../program/language/capture.wave"),
    ] {
        let mut runtime = Runtime::new(&frontend::lowering::parse(source).unwrap());
        runtime.run(12000, Limit::default());
        assert!(!runtime.event.is_empty());
        for event in &runtime.event {
            let identity = Flow::identity(&runtime.state[event.identity.source]);
            assert_eq!(identity.compose(&event.flow), *event.flow);
            assert!(runtime.view.iter().any(|view| {
                view.source == event.identity.source
                    && view.target == event.target
                    && *view.flow == *event.flow
            }));
        }
        assert!(runtime.event.iter().any(|event| {
            runtime
                .view
                .iter()
                .any(|view| Arc::ptr_eq(&event.flow, &view.flow))
        }));
    }
}
