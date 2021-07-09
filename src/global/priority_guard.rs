use tokio::sync::{broadcast, mpsc};

use crate::component::ComponentName;

use super::{InputMessage, InputMessageData, InputSourceHandle, Message};

enum SendingChannel {
    Mpsc(mpsc::Sender<InputMessage>),
    Broadcast(broadcast::Sender<InputMessage>),
}

impl SendingChannel {
    pub async fn send(&self, message: InputMessage) {
        match self {
            SendingChannel::Mpsc(tx) => tx.send(message).await.ok(),
            SendingChannel::Broadcast(tx) => tx.send(message).ok().map(|_| ()),
        };
    }
}

impl From<mpsc::Sender<InputMessage>> for SendingChannel {
    fn from(tx: mpsc::Sender<InputMessage>) -> Self {
        Self::Mpsc(tx)
    }
}

impl From<broadcast::Sender<InputMessage>> for SendingChannel {
    fn from(tx: broadcast::Sender<InputMessage>) -> Self {
        Self::Broadcast(tx)
    }
}

pub struct PriorityGuard {
    channel: SendingChannel,
    source_id: usize,
    priority: Option<i32>,
    component: ComponentName,
}

impl PriorityGuard {
    pub fn new_mpsc(
        tx: mpsc::Sender<InputMessage>,
        handle: &InputSourceHandle<InputMessage>,
    ) -> Self {
        Self {
            channel: SendingChannel::from(tx),
            source_id: handle.id(),
            priority: handle.priority(),
            component: handle.name().component(),
        }
    }

    pub fn new_broadcast(handle: &InputSourceHandle<InputMessage>) -> Self {
        Self {
            channel: SendingChannel::from(handle.channel().clone()),
            source_id: handle.id(),
            priority: handle.priority(),
            component: handle.name().component(),
        }
    }

    pub fn set_priority(&mut self, priority: Option<i32>) {
        self.priority = priority;
    }
}

impl Drop for PriorityGuard {
    fn drop(&mut self) {
        if let Some(priority) = self.priority {
            futures::executor::block_on(async {
                self.channel
                    .send(InputMessage::new(
                        self.source_id,
                        self.component,
                        InputMessageData::Clear { priority },
                    ))
                    .await;
            })
        }
    }
}
