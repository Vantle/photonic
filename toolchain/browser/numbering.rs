use photonic::snapshot::{Node, Snapshot, Token, Value};
use photonic::status::Status;
use spectrum::Event;
use spectrum::configuration::{Coherence, Configuration, Frame, Occurrence, Opener};
use spectrum::numbering::Numbering;

// The numbering reads a run as Spectrum records one, so every configuration and event here takes
// the handle Spectrum's answers give it.
fn occurrence(token: &Token) -> Occurrence {
    Occurrence {
        id: token.id,
        value: match &token.value {
            Value::Atom(atom) => spectrum::configuration::Value::Atom(atom.to_string()),
            Value::Rule(rule) => spectrum::configuration::Value::Rule(*rule),
        },
    }
}

fn configuration(node: &Node) -> Configuration {
    Configuration {
        coherence: node
            .world
            .iter()
            .map(|world| Coherence {
                frame: world.frame,
                occurrence: world.particle.iter().map(occurrence).collect(),
            })
            .collect(),
        frame: node
            .frame
            .iter()
            .map(|frame| Frame {
                opener: match (frame.opener, frame.parent) {
                    (Some(rule), _) => Some(Opener::Rule(rule)),
                    (None, Some(_)) => Some(Opener::Program),
                    (None, None) => None,
                },
                parent: frame.parent,
                lexical: frame.lexical,
                rule: frame.particle.iter().map(occurrence).collect(),
                held: frame.held.iter().map(occurrence).collect(),
            })
            .collect(),
        supported: node.status == Status::Supported,
    }
}

pub fn new(snapshot: &Snapshot) -> Numbering {
    let configuration = snapshot.state.iter().map(configuration).collect::<Vec<_>>();
    let event = snapshot
        .event
        .iter()
        .map(|event| Event {
            source: event.source,
            target: event.target,
            rule: event.rule,
            supported: event.status == Status::Supported,
            footprint: event.footprint.clone(),
            exact: event.exact.clone(),
            read: event.read.clone(),
            world: event.world.clone(),
            deduction: snapshot.deduction(event.id),
        })
        .collect::<Vec<_>>();
    Numbering::new(&configuration, &event)
}
