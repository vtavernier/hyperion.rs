use std::collections::{BTreeMap, HashMap};
use std::pin::Pin;
use std::time::Instant;

use futures::Future;
use tokio::select;

use crate::{
    api::types::PriorityInfo,
    component::ComponentName,
    global::{Global, InputMessage, InputMessageData, Message},
    models::Color,
};

mod muxed_message;
pub use muxed_message::*;

#[derive(Debug)]
struct InputEntry {
    input_id: usize,
    message: InputMessage,
    expires: Option<Instant>,
}

pub struct PriorityMuxer {
    global: Global,
    inputs: BTreeMap<i32, InputEntry>,
    input_id: usize,
    timeouts: HashMap<
        usize,
        Box<dyn Fn() -> Pin<Box<dyn Future<Output = (usize, i32)> + Send + Sync>> + Send + Sync>,
    >,
}

pub const MAX_PRIORITY: i32 = 256;
const MUXER_ID: usize = 0;

impl PriorityMuxer {
    pub async fn new(global: Global) -> Self {
        let mut this = Self {
            global,
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

    fn notify_output_change(&mut self) -> MuxedMessage {
        // unwrap: there is always at least one input
        let target = self.inputs.values().next().unwrap();

        MuxedMessage::new(target.message.data().clone().into())
    }

    fn insert_input(&mut self, priority: i32, input: InputMessage) {
        // Get the duration of this input
        let expires = input
            .data()
            .duration()
            .map(|duration| Instant::now() + duration.to_std().unwrap());

        // Insert the input, replacing the old one
        let before = self.inputs.insert(
            priority,
            InputEntry {
                input_id: self.input_id,
                message: input,
                expires,
            },
        );

        // Drop the future for the previous input
        if let Some(InputEntry { input_id, .. }) = before {
            self.timeouts.remove(&input_id);
        }

        // Add the future for the current input
        if let Some(expires) = expires {
            let id = self.input_id;

            self.timeouts.insert(
                self.input_id,
                Box::new(move || {
                    Box::pin(async move {
                        tokio::time::sleep_until(expires.into()).await;
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
        if let Some(InputEntry { input_id, .. }) = self.inputs.remove(&priority) {
            self.timeouts.remove(&input_id);
            true
        } else {
            false
        }
    }

    async fn clear_all(&mut self) -> MuxedMessage {
        self.clear_inputs();
        debug!("cleared all inputs");

        self.insert_input(
            MAX_PRIORITY,
            InputMessage::new(
                MUXER_ID,
                ComponentName::All,
                InputMessageData::SolidColor {
                    priority: MAX_PRIORITY,
                    duration: None,
                    color: Color::from_components((0, 0, 0)),
                },
            ),
        );

        debug!(priority = %self.current_priority(), "current priority changed");
        self.notify_output_change()
    }

    async fn clear(&mut self, priority: i32) -> Option<MuxedMessage> {
        assert!(priority < MAX_PRIORITY);
        let mut notify = self.current_priority() == priority;

        notify = self.clear_input(priority) && notify;
        debug!(priority = %priority, "cleared priority");

        if notify {
            debug!(priority = %self.current_priority(), "current priority changed");
            Some(self.notify_output_change())
        } else {
            None
        }
    }

    async fn handle_input(&mut self, input: InputMessage) -> Option<MuxedMessage> {
        let priority = input.data().priority().unwrap();
        let is_new = priority < self.current_priority();
        let notify = priority <= self.current_priority();

        let before = self.insert_input(priority, input.clone());
        trace!(
            priority = %priority,
            after = ?input,
            before = ?before,
            "new command for priority level",
        );

        if is_new {
            debug!(priority = %priority, "current priority changed");
        }

        if notify {
            Some(self.notify_output_change())
        } else {
            None
        }
    }

    async fn handle_timeout(&mut self, (id, priority): (usize, i32)) -> Option<MuxedMessage> {
        let current_priority = self.current_priority();

        // Check if the input for the target priority is still the one mentioned in the future
        if let Some(input) = self.inputs.get(&priority) {
            if input.input_id == id {
                if let Some(removed) = self.inputs.remove(&priority) {
                    debug!(input = ?removed, "input timeout");
                }
            } else {
                warn!(id = %id, "unexpected timeout for input");
            }
        }

        // Remove the future
        self.timeouts.remove(&id);

        // If the timeout priority is <=, then it was the current input
        if current_priority >= priority {
            debug!(priority = %current_priority, "current priority changed");
            Some(self.notify_output_change())
        } else {
            None
        }
    }

    pub async fn handle_message(&mut self, input: InputMessage) -> Option<MuxedMessage> {
        trace!(input = ?input, "got input");

        // Check if this will change the output
        match input.data() {
            InputMessageData::ClearAll => Some(self.clear_all().await),
            InputMessageData::Clear { priority } => self.clear(*priority).await,
            _ => self.handle_input(input).await,
        }
    }

    pub async fn current_priorities(&self) -> Vec<PriorityInfo> {
        self.global
            .read_input_sources(|sources| {
                // Inputs are sorted by priority, so i == 0 denotes the
                // current (active) entry
                self.inputs
                    .values()
                    .enumerate()
                    .map(|(i, entry)| {
                        PriorityInfo::new(
                            &entry.message,
                            sources
                                .get(&entry.message.source_id())
                                .map(|source| source.name().to_string())
                                .unwrap_or_else(String::new),
                            entry.expires,
                            i == 0,
                        )
                    })
                    .collect()
            })
            .await
    }

    pub async fn update(&mut self) -> Option<MuxedMessage> {
        if self.timeouts.len() > 0 {
            select! {
                id = futures::future::select_all(self.timeouts.values().map(|f| f())) => {
                    return self.handle_timeout(id.0).await;
                },
            }
        } else {
            futures::future::pending::<()>().await;
            None
        }
    }
}
