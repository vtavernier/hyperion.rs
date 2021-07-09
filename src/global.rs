use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;

use parse_display::Display;
use tokio::sync::broadcast;
use tokio::sync::RwLock;

mod event;
pub use event::*;

mod hook_runner;
pub use hook_runner::*;

mod input_message;
pub use input_message::*;

mod input_source;
pub use input_source::*;

mod paths;
pub use paths::*;

use crate::{component::ComponentName, instance::InstanceHandle, models::Config};

pub trait Message: Sized {
    type Data;

    fn new(source_id: usize, component: ComponentName, data: Self::Data) -> Self;

    fn source_id(&self) -> usize;

    fn component(&self) -> ComponentName;

    fn data(&self) -> &Self::Data;

    fn unregister_source(global: &mut GlobalData, input_source: &InputSource<Self>);
}

#[derive(Clone)]
pub struct Global(Arc<RwLock<GlobalData>>);

#[derive(Display, Debug)]
pub enum InputSourceName {
    #[display("Boblight({peer_addr})")]
    Boblight { peer_addr: SocketAddr },
    #[display("FlatBuffers({peer_addr}): {origin}")]
    FlatBuffers {
        peer_addr: SocketAddr,
        origin: String,
    },
    #[display("JSON({peer_addr})")]
    Json { peer_addr: SocketAddr },
    #[display("Protobuf({peer_addr})")]
    Protobuf { peer_addr: SocketAddr },
    #[display("Web({peer_addr})")]
    Web { peer_addr: SocketAddr },
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

        Ok(InputSourceHandle::new(
            self.0.write().await.register_input_source(name, priority),
            self.clone(),
        ))
    }

    pub async fn subscribe_input(&self) -> broadcast::Receiver<InputMessage> {
        self.0.read().await.input_tx.subscribe()
    }

    pub async fn register_instance(&self, handle: InstanceHandle) {
        self.0.write().await.register_instance(handle);
    }

    pub async fn unregister_instance(&self, id: i32) {
        self.0.write().await.unregister_instance(id);
    }

    pub async fn get_instance(&self, id: i32) -> Option<InstanceHandle> {
        self.0.read().await.instances.get(&id).cloned()
    }

    pub async fn default_instance(&self) -> Option<(i32, InstanceHandle)> {
        self.0
            .read()
            .await
            .instances
            .iter()
            .next()
            .map(|(k, v)| (*k, v.clone()))
    }

    pub async fn read_config<T>(&self, f: impl FnOnce(&Config) -> T) -> T {
        let data = self.0.read().await;
        f(&data.config)
    }

    pub async fn read_input_sources<T>(
        &self,
        f: impl FnOnce(&HashMap<usize, Arc<InputSource<InputMessage>>>) -> T,
    ) -> T {
        let data = self.0.read().await;
        f(&data.input_sources)
    }

    pub async fn get_event_tx(&self) -> broadcast::Sender<Event> {
        self.0.read().await.event_tx.clone()
    }

    pub async fn subscribe_events(&self) -> broadcast::Receiver<Event> {
        self.0.read().await.event_tx.subscribe()
    }
}

pub struct GlobalData {
    input_tx: broadcast::Sender<InputMessage>,
    input_sources: HashMap<usize, Arc<InputSource<InputMessage>>>,
    next_input_source_id: usize,
    config: Config,
    instances: BTreeMap<i32, InstanceHandle>,
    event_tx: broadcast::Sender<Event>,
}

impl GlobalData {
    pub fn new(config: &Config) -> Self {
        let (input_tx, _) = broadcast::channel(4);
        let (event_tx, _) = broadcast::channel(4);

        Self {
            input_tx,
            input_sources: Default::default(),
            next_input_source_id: 1,
            config: config.clone(),
            instances: Default::default(),
            event_tx,
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

        let input_source = Arc::new(InputSource::new(id, name, priority, self.input_tx.clone()));

        info!(source = %input_source, "registered new input source");

        self.input_sources.insert(id, input_source.clone());

        input_source
    }

    fn unregister_input_source(&mut self, source: &InputSource<InputMessage>) {
        if let Some(is) = self.input_sources.remove(&source.id()) {
            info!(source = %*is, "unregistered input source");
        }
    }

    fn register_instance(&mut self, handle: InstanceHandle) {
        let id = handle.id();
        self.instances.insert(id, handle);
        info!(id = %id, "registered instance");
    }

    fn unregister_instance(&mut self, id: i32) {
        if let Some(_) = self.instances.remove(&id) {
            info!(id = %id, "unregistered instance");
        }
    }
}
