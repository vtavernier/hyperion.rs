use std::sync::Arc;

use parse_display::Display;
use thiserror::Error;
use tokio::sync::broadcast;

use super::{Global, InputSourceName, Message};

#[derive(Display)]
#[display("`{name}` (id = {id}, priority = {priority:?})")]
pub struct InputSource<T: Message> {
    pub(super) id: usize,
    pub(super) name: InputSourceName,
    pub(super) priority: Option<i32>,
    pub(super) tx: broadcast::Sender<T>,
}

impl<T: Message> InputSource<T> {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn priority(&self) -> Option<i32> {
        self.priority
    }

    pub fn send(&self, message: T::Data) -> Result<usize, broadcast::error::SendError<T>> {
        self.tx.send(T::new(self.id, message))
    }
}

pub struct InputSourceHandle<T: Message> {
    pub(super) input_source: Arc<InputSource<T>>,
    pub(super) global: Global,
}

impl<T: Message> std::ops::Deref for InputSourceHandle<T> {
    type Target = InputSource<T>;

    fn deref(&self) -> &Self::Target {
        &*self.input_source
    }
}

impl<T: Message> Drop for InputSourceHandle<T> {
    fn drop(&mut self) {
        // TODO: Can this block?
        futures::executor::block_on(async {
            T::unregister_source(&mut *self.global.0.write().await, &*self.input_source);
        });
    }
}

#[derive(Debug, Error)]
pub enum InputSourceError {
    #[error("invalid priority: {0}")]
    InvalidPriority(i32),
}
