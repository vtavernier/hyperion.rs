use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use parse_display::Display;
use tokio::sync::broadcast;
use tokio::sync::RwLock;

mod input_message;
pub use input_message::*;

mod input_source;
pub use input_source::*;

mod muxed_message;
pub use muxed_message::*;

use crate::models::Config;

pub trait Message: Sized {
    type Data;

    fn new(source_id: usize, data: Self::Data) -> Self;

    fn data(&self) -> &Self::Data;

    fn unregister_source(global: &mut GlobalData, input_source: &InputSource<Self>);
}

#[derive(Clone)]
pub struct Global(Arc<RwLock<GlobalData>>);

#[derive(Display, Debug)]
pub enum InputSourceName {
    #[display("FlatBuffers({peer_addr}): {origin}")]
    FlatBuffers {
        peer_addr: SocketAddr,
        origin: String,
    },
    #[display("JSON({peer_addr})")]
    Json { peer_addr: SocketAddr },
    #[display("Protobuf({peer_addr})")]
    Protobuf { peer_addr: SocketAddr },
    #[display("PriorityMuxer")]
    PriorityMuxer,
}

impl Global {
    pub async fn register_input_source(
        &self,
        name: InputSourceName,
        priority: Option<i32>,
    ) -> Result<InputSourceHandle<InputMessage>, InputSourceError> {
        let priority = if let Some(priority) = priority {
            if priority < 0 || priority > 255 {
                return Err(InputSourceError::InvalidPriority(priority));
            }

            Some(priority)
        } else {
            // TODO: Default value?
            None
        };

        Ok(InputSourceHandle {
            input_source: self.0.write().await.register_input_source(name, priority),
            global: self.clone(),
        })
    }

    pub async fn register_muxed_source(
        &self,
        name: InputSourceName,
    ) -> Result<InputSourceHandle<MuxedMessage>, InputSourceError> {
        Ok(InputSourceHandle {
            input_source: self.0.write().await.register_muxed_source(name),
            global: self.clone(),
        })
    }

    pub async fn subscribe_input(&self) -> broadcast::Receiver<InputMessage> {
        self.0.read().await.input_tx.subscribe()
    }

    pub async fn subscribe_muxed(&self) -> broadcast::Receiver<MuxedMessage> {
        self.0.read().await.muxed_tx.subscribe()
    }

    pub async fn read_config<T>(&self, f: impl FnOnce(&Config) -> T) -> T {
        let data = self.0.read().await;
        f(&data.config)
    }
}

pub struct GlobalData {
    input_tx: broadcast::Sender<InputMessage>,
    muxed_tx: broadcast::Sender<MuxedMessage>,
    input_sources: HashMap<usize, Arc<InputSource<InputMessage>>>,
    next_input_source_id: usize,
    muxed_sources: HashMap<usize, Arc<InputSource<MuxedMessage>>>,
    next_muxed_source_id: usize,
    config: Config,
}

impl GlobalData {
    pub fn new(config: &Config) -> Self {
        let (input_tx, _) = broadcast::channel(4);
        let (muxed_tx, _) = broadcast::channel(4);

        Self {
            input_tx,
            muxed_tx,
            input_sources: Default::default(),
            next_input_source_id: 1,
            muxed_sources: Default::default(),
            next_muxed_source_id: 1,
            config: config.clone(),
        }
    }

    pub fn wrap(self) -> Global {
        Global(Arc::new(RwLock::new(self)))
    }

    fn register_input_source(
        &mut self,
        name: InputSourceName,
        priority: Option<i32>,
    ) -> Arc<InputSource<InputMessage>> {
        let id = self.next_input_source_id;
        self.next_input_source_id += 1;

        let input_source = Arc::new(InputSource {
            id,
            name,
            priority,
            tx: self.input_tx.clone(),
        });

        info!("registered new input source {}", *input_source);

        self.input_sources.insert(id, input_source.clone());

        input_source
    }

    fn unregister_input_source(&mut self, source: &InputSource<InputMessage>) {
        if let Some(is) = self.input_sources.remove(&source.id) {
            info!("unregistered input source {}", *is);
        }
    }

    fn register_muxed_source(&mut self, name: InputSourceName) -> Arc<InputSource<MuxedMessage>> {
        let id = self.next_muxed_source_id;
        self.next_muxed_source_id += 1;

        let input_source = Arc::new(InputSource {
            id,
            name,
            priority: None,
            tx: self.muxed_tx.clone(),
        });

        info!("registered new muxed source {}", *input_source);

        self.muxed_sources.insert(id, input_source.clone());

        input_source
    }

    fn unregister_muxed_source(&mut self, source: &InputSource<MuxedMessage>) {
        if let Some(is) = self.muxed_sources.remove(&source.id) {
            info!("unregistered muxed source {}", *is);
        }
    }
}
