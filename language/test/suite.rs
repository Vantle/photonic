mod addition;
mod assignment;
mod binary;
mod executor;
mod group;
mod import;
mod joining;
mod membership;
mod metaprogram;
mod multiplication;
mod natural;
mod path;
mod position;
mod prism;
mod product;
mod reachability;
mod reduction;
mod report;
mod runtime;
mod scheduling;
mod sequence;
mod support;
mod vacancy;

pub(crate) fn target(program: &frontend::source::Program, text: &str) -> frontend::source::Program {
    let mut target = frontend::lowering::parse(text).unwrap();
    target.preserve(program);
    target
}

pub(crate) fn atom(token: &crate::snapshot::Token) -> Option<&str> {
    match &token.value {
        crate::snapshot::Value::Atom(atom) => Some(atom),
        crate::snapshot::Value::Rule(_) => None,
    }
}

pub(crate) fn executor(worker: usize) -> crate::executor::Executor {
    crate::executor::Executor::new(std::num::NonZeroUsize::new(worker).unwrap()).unwrap()
}

pub(crate) fn advance(
    index: &mut crate::index::Index,
    state: std::sync::Arc<crate::state::State>,
    removed: &crate::basis::Set<usize>,
) {
    let change = crate::change::Change {
        world: removed.clone(),
        insertion: index.state.world.len() - removed.len()..state.world.len(),
        frame: (0..index.state.frame.len().max(state.frame.len()))
            .filter(|&frame| index.state.frame.get(frame) != state.frame.get(frame))
            .collect(),
    };
    index.update(state, &change);
}
