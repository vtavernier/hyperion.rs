use std::{collections::BTreeMap, fmt::Display, sync::Arc};

use tokio::sync::broadcast;

use super::{Event, InstanceEvent, InstanceEventKind};
use crate::models::Hooks;

const INSTANCE_ID: &'static str = "HYPERION_INSTANCE_ID";

struct HookBuilder<'s> {
    variables: BTreeMap<&'static str, String>,
    command: &'s Vec<String>,
}

impl<'s> HookBuilder<'s> {
    pub fn new(command: &'s Vec<String>) -> Self {
        Self {
            variables: Default::default(),
            command,
        }
    }

    pub fn arg(mut self, k: &'static str, v: impl Display) -> Self {
        self.variables.insert(k, v.to_string());
        self
    }

    pub async fn run(self) -> Option<Result<(), std::io::Error>> {
        if self.command.is_empty() {
            return None;
        }

        let mut process = tokio::process::Command::new(&self.command[0]);
        process.args(&self.command[1..]);
        process.envs(self.variables);

        debug!(command = ?self.command, "spawning hook");

        Some(process.spawn().map(|_| {
            // Drop child
        }))
    }
}

#[derive(Debug)]
pub struct HookRunner {
    event_rx: broadcast::Receiver<Event>,
    config: Arc<Hooks>,
}

impl HookRunner {
    pub fn new(hooks: Hooks, event_rx: broadcast::Receiver<Event>) -> Self {
        Self {
            config: Arc::new(hooks),
            event_rx,
        }
    }

    async fn handle_message(&self, message: &Event) -> Option<Result<(), std::io::Error>> {
        match message {
            Event::Start => HookBuilder::new(&self.config.start).run(),
            Event::Stop => HookBuilder::new(&self.config.stop).run(),
            Event::Instance(InstanceEvent { id, kind }) => match kind {
                InstanceEventKind::Start => HookBuilder::new(&self.config.instance_start),
                InstanceEventKind::Stop => HookBuilder::new(&self.config.instance_stop),
                InstanceEventKind::Activate => HookBuilder::new(&self.config.instance_activate),
                InstanceEventKind::Deactivate => HookBuilder::new(&self.config.instance_deactivate),
            }
            .arg(INSTANCE_ID, id)
            .run(),
        }
        .await
    }

    pub async fn run(mut self) {
        loop {
            match self.event_rx.recv().await {
                Ok(message) => {
                    match self.handle_message(&message).await {
                        Some(result) => {
                            match result {
                                Ok(()) => { // Nothing to notify, hook spawned successfully
                                }
                                Err(error) => {
                                    warn!(error = %error, event = ?message, "hook error");
                                }
                            }
                        }
                        None => {
                            // No hook for this event
                        }
                    }
                }
                Err(error) => match error {
                    broadcast::error::RecvError::Closed => {
                        break;
                    }
                    broadcast::error::RecvError::Lagged(skipped) => {
                        warn!(skipped = %skipped, "hook runner missed events");
                    }
                },
            }
        }
    }
}
