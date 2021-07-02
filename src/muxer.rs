use std::collections::{BTreeMap, HashMap};
use std::pin::Pin;

use futures::Future;
use tokio::select;
use tokio::sync::broadcast::Receiver;

use crate::{
    global::{
        Global, InputMessage, InputMessageData, InputSourceHandle, InputSourceName, Message,
        MuxedMessage,
    },
    models::Color,
};

pub struct PriorityMuxer {
    receiver: Receiver<InputMessage>,
    source_handle: InputSourceHandle<MuxedMessage>,
    inputs: BTreeMap<i32, (usize, InputMessage)>,
    input_id: usize,
    timeouts: HashMap<
        usize,
        Box<dyn Fn() -> Pin<Box<dyn Future<Output = (usize, i32)> + Send + Sync>> + Send + Sync>,
    >,
}

const MAX_PRIORITY: i32 = 256;

impl PriorityMuxer {
    pub async fn new(global: Global) -> Self {
        let mut this = Self {
            receiver: global.subscribe_input().await,
            source_handle: global
                .register_muxed_source(InputSourceName::PriorityMuxer)
                .await
                .unwrap(),
            inputs: Default::default(),
            timeouts: Default::default(),
            input_id: 0,
        };

        // Start by clearing all outputs
        this.clear_all().await;

        this
    }

    fn current_priority(&self) -> i32 {
        *self.inputs.keys().next().unwrap()
    }

    fn notify_output_change(&mut self) {
        // unwrap: there is always at least one input
        let target = self.inputs.values().next().unwrap();

        match self.source_handle.send(target.1.data().clone().into()) {
            Ok(_) => {}
            Err(error) => {
                warn!("error forwarding muxed message: {:?}", error);
            }
        }
    }

    fn insert_input(&mut self, priority: i32, input: InputMessage) {
        // Get the duration of this input
        let duration = input.data().duration();

        // Insert the input, replacing the old one
        let before = self.inputs.insert(priority, (self.input_id, input));

        // Drop the future for the previous input
        if let Some((id, _)) = before {
            self.timeouts.remove(&id);
        }

        // Add the future for the current input
        if let Some(duration) = duration {
            let id = self.input_id;
            let until = std::time::Instant::now() + duration.to_std().unwrap();

            self.timeouts.insert(
                self.input_id,
                Box::new(move || {
                    Box::pin(async move {
                        tokio::time::sleep_until(until.into()).await;
                        (id, priority)
                    })
                }),
            );
        }

        // Increment id
        self.input_id += 1;
    }

    fn clear_inputs(&mut self) {
        self.inputs.clear();
        self.timeouts.clear();
    }

    fn clear_input(&mut self, priority: i32) -> bool {
        if let Some((id, _)) = self.inputs.remove(&priority) {
            self.timeouts.remove(&id);
            true
        } else {
            false
        }
    }

    async fn clear_all(&mut self) {
        self.clear_inputs();
        debug!("cleared all inputs");

        self.insert_input(
            MAX_PRIORITY,
            InputMessage::new(
                self.source_handle.id(),
                InputMessageData::SolidColor {
                    priority: MAX_PRIORITY,
                    duration: None,
                    color: Color::from_components((0, 0, 0)),
                },
            ),
        );

        debug!("current priority is now {}", self.current_priority());
        self.notify_output_change()
    }

    async fn clear(&mut self, priority: i32) {
        assert!(priority < MAX_PRIORITY);
        let mut notify = self.current_priority() == priority;

        notify = self.clear_input(priority) && notify;
        debug!("cleared priority {}", priority);

        if notify {
            debug!("current priority is now {}", self.current_priority());
            self.notify_output_change()
        }
    }

    async fn handle_input(&mut self, input: InputMessage) {
        let priority = input.data().priority().unwrap();
        let is_new = priority < self.current_priority();
        let notify = priority <= self.current_priority();

        let before = self.insert_input(priority, input.clone());
        trace!(
            "new command for priority {}: {:?}, replaces: {:?}",
            priority,
            input,
            before
        );

        if is_new {
            debug!("current priority is now {}", priority);
        }

        if notify {
            self.notify_output_change()
        }
    }

    async fn handle_timeout(&mut self, (id, priority): (usize, i32)) {
        let current_priority = self.current_priority();

        // Check if the input for the target priority is still the one mentioned in the future
        if let Some(input) = self.inputs.get(&priority) {
            if input.0 == id {
                if let Some(removed) = self.inputs.remove(&priority) {
                    debug!("timeout for input {:?}", removed);
                }
            } else {
                warn!("unexpected timeout for input id {}", id);
            }
        }

        // Remove the future
        self.timeouts.remove(&id);

        // If the timeout priority is <=, then it was the current input
        if current_priority >= priority {
            debug!("current priority is now {}", self.current_priority());
            self.notify_output_change();
        }
    }

    async fn handle_input_recv(
        &mut self,
        input: Result<InputMessage, tokio::sync::broadcast::error::RecvError>,
    ) {
        match input {
            Ok(input) => {
                trace!("got input: {:?}", input);

                // Check if this will change the output
                match input.data() {
                    InputMessageData::ClearAll => self.clear_all().await,
                    InputMessageData::Clear { priority } => self.clear(*priority).await,
                    InputMessageData::SolidColor { .. } => self.handle_input(input).await,
                    InputMessageData::Image { .. } => self.handle_input(input).await,
                    InputMessageData::PrioritiesRequest { response } => {
                        response.lock().unwrap().take().map(move |channel| {
                            channel.send(self.inputs.values().map(|(_, x)| x).cloned().collect())
                        });
                    }
                }
            }
            Err(error) => {
                error!("could not get input: {:?}", error);
            }
        }
    }

    pub async fn run(mut self) {
        loop {
            if self.timeouts.len() > 0 {
                select! {
                    id = futures::future::select_all(self.timeouts.values().map(|f| f())) => {
                        self.handle_timeout(id.0).await;
                    },
                    recv = self.receiver.recv() => {
                        self.handle_input_recv(recv).await;
                    }
                };
            } else {
                let recv = self.receiver.recv().await;
                self.handle_input_recv(recv).await;
            }
        }
    }
}
