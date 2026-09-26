use crate::play::Shared;
use gpu::engine::Engine;
use network::input::{Input, Output};
use network::model::Model;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::time::Duration;

const LIMIT: usize = 1_024;

pub struct Request {
    input: Arc<Vec<Input>>,
    reply: SyncSender<Option<Vec<Output>>>,
}

pub struct Server {
    sender: Sender<Request>,
}

impl Server {
    pub fn new() -> (Self, Receiver<Request>) {
        let (sender, receiver) = mpsc::channel();
        (Self { sender }, receiver)
    }

    pub fn infer(&self, input: Arc<Vec<Input>>) -> Option<Vec<Output>> {
        let (reply, answer) = mpsc::sync_channel(1);
        self.sender.send(Request { input, reply }).ok()?;
        answer.recv().ok().flatten()
    }
}

pub fn serve(mut engine: Engine, receiver: &Receiver<Request>, shared: &Shared) {
    let mut loaded: Option<Arc<Model>> = None;
    while !shared.stop.load(Ordering::Relaxed) {
        let first = match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(request) => request,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        let mut size = first.input.len();
        let mut batch = vec![first];
        while size < LIMIT {
            let Ok(request) = receiver.recv_timeout(Duration::from_micros(200)) else {
                break;
            };
            size += request.input.len();
            batch.push(request);
        }
        let model = shared
            .model
            .read()
            .expect("the model lock is never poisoned")
            .clone();
        if loaded
            .as_ref()
            .is_none_or(|known| !Arc::ptr_eq(known, &model))
        {
            if engine.load(model.parameter()).is_err() {
                for request in batch {
                    request.reply.send(None).ok();
                }
                continue;
            }
            loaded = Some(model);
        }
        let input = batch
            .iter()
            .flat_map(|request| request.input.iter())
            .collect::<Vec<_>>();
        let Ok(output) = engine.infer(&input) else {
            for request in batch {
                request.reply.send(None).ok();
            }
            continue;
        };
        let mut output = output.into_iter();
        for request in batch {
            let part = output.by_ref().take(request.input.len()).collect();
            request.reply.send(Some(part)).ok();
        }
    }
}
