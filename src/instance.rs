use std::sync::Arc;

use thiserror::Error;
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
};

use crate::api::types::PriorityInfo;
use crate::models::Color;
use crate::{
    global::{Global, InputMessage},
    models::InstanceConfig,
    servers::{self, ServerHandle},
};

mod black_border_detector;
use black_border_detector::*;

mod core;
use self::core::*;

mod device;
use device::*;

mod muxer;
use muxer::*;

mod smoothing;
use smoothing::*;

#[derive(Debug, Error)]
pub enum InstanceError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("device error: {0}")]
    Device(#[from] DeviceError),
    #[error("recv error: {0}")]
    Recv(#[from] broadcast::error::RecvError),
}

pub struct Instance {
    config: Arc<InstanceConfig>,
    device: InstanceDevice,
    handle_rx: mpsc::Receiver<InstanceMessage>,
    receiver: broadcast::Receiver<InputMessage>,
    local_receiver: mpsc::Receiver<InputMessage>,
    muxer: PriorityMuxer,
    core: Core,
    _boblight_server: Option<Result<ServerHandle, std::io::Error>>,
}

impl Instance {
    pub async fn new(global: Global, config: InstanceConfig) -> (Self, InstanceHandle) {
        let device: InstanceDevice =
            Device::new(&config.instance.friendly_name, config.device.clone())
                .await
                .into();

        if let Err(error) = &device.inner {
            error!(
                instance = %config.instance.id,
                name = %config.instance.friendly_name,
                error = %error,
                "initializing instance failed"
            );
        }

        let receiver = global.subscribe_input().await;
        let (local_tx, local_receiver) = mpsc::channel(4);

        let muxer = PriorityMuxer::new(global.clone()).await;
        let core = Core::new(&config).await;

        let (tx, handle_rx) = mpsc::channel(1);
        let id = config.instance.id;
        let handle = InstanceHandle { id, tx, local_tx };

        let config = Arc::new(config);
        let _boblight_server = if config.boblight_server.enable {
            let server_handle = servers::bind(
                "Boblight",
                config.boblight_server.clone(),
                global.clone(),
                {
                    let led_count = config.leds.leds.len();
                    let handle = handle.clone();

                    move |tcp, global| {
                        servers::boblight::handle_client(tcp, led_count, handle.clone(), global)
                    }
                },
            )
            .await;

            if let Err(error) = &server_handle {
                error!(
                    instance = %config.instance.id,
                    name = %config.instance.friendly_name,
                    error = %error,
                    "cannot start Boblight server"
                );
            }

            Some(server_handle)
        } else {
            None
        };

        (
            Self {
                config,
                device,
                handle_rx,
                receiver,
                local_receiver,
                muxer,
                core,
                _boblight_server,
            },
            handle,
        )
    }

    async fn on_input_message(&mut self, message: InputMessage) {
        if let Some(message) = self.muxer.handle_message(message).await {
            // The message triggered a muxing update
            self.core.handle_message(message);
        }
    }

    pub fn id(&self) -> i32 {
        self.config.instance.id
    }

    async fn handle_instance_message(&mut self, message: InstanceMessage) {
        // ok: the instance shouldn't care if the receiver dropped

        match message {
            InstanceMessage::PriorityInfo(tx) => {
                tx.send(self.muxer.current_priorities().await).ok();
            }
            InstanceMessage::Config(tx) => {
                tx.send(self.config.clone()).ok();
            }
        }
    }

    #[instrument]
    pub async fn run(mut self) -> Result<(), InstanceError> {
        loop {
            select! {
                update = self.device.update() => {
                    trace!("device update");

                    if let Err(error) = update {
                        // A device update shouldn't error, disable it
                        error!(error = %error, "device update failed, disabling device");
                        self.device.inner = Err(error);
                    }
                },
                message = self.receiver.recv() => {
                    trace!(message = ?message, "global msg");

                    match message {
                        Ok(message) => {
                            self.on_input_message(message).await;
                        },
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            // No more input messages
                            break Ok(());
                        },
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!(skipped = %skipped, "skipped input messages");
                        },
                    }
                },
                message = self.local_receiver.recv() => {
                    trace!(message = ?message, "local msg");

                    if let Some(message) = message {
                        self.on_input_message(message).await;
                    } else {
                        break Ok(());
                    }
                },
                message = self.muxer.update() => {
                    trace!(message = ?message, "muxer msg");

                    // Muxer update completed
                    if let Some(message) = message {
                        self.core.handle_message(message);
                    }
                },
                led_data = self.core.update() => {
                    // LED data changed
                    self.device.set_led_data(led_data).await?;

                    trace!("core update");
                },
                message = self.handle_rx.recv() => {
                    trace!(message = ?message, "handle_rx msg");

                    if let Some(message) = message {
                        self.handle_instance_message(message).await;
                    } else {
                        // If the handle is dropped, it means the instance was unregistered
                        break Ok(());
                    }
                }
            }
        }
    }
}

impl std::fmt::Debug for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Instance").field("id", &self.id()).finish()
    }
}

/// A wrapper for a device that may have failed initializing
struct InstanceDevice {
    inner: Result<Device, DeviceError>,
}

impl InstanceDevice {
    async fn update(&mut self) -> Result<(), DeviceError> {
        if let Ok(device) = &mut self.inner {
            device.update().await
        } else {
            futures::future::pending::<()>().await;
            Ok(())
        }
    }

    async fn set_led_data(&mut self, led_data: &[Color]) -> Result<(), DeviceError> {
        if let Ok(device) = &mut self.inner {
            device.set_led_data(led_data).await
        } else {
            Ok(())
        }
    }
}

impl From<Result<Device, DeviceError>> for InstanceDevice {
    fn from(inner: Result<Device, DeviceError>) -> Self {
        Self { inner }
    }
}

#[derive(Debug)]
enum InstanceMessage {
    PriorityInfo(oneshot::Sender<Vec<PriorityInfo>>),
    Config(oneshot::Sender<Arc<InstanceConfig>>),
}

#[derive(Clone)]
pub struct InstanceHandle {
    id: i32,
    tx: mpsc::Sender<InstanceMessage>,
    local_tx: mpsc::Sender<InputMessage>,
}

#[derive(Debug, Error)]
pub enum InstanceHandleError {
    #[error("the corresponding instance is no longer running")]
    Dropped,
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for InstanceHandleError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::Dropped
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for InstanceHandleError {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::Dropped
    }
}

impl InstanceHandle {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub async fn send(&self, input: InputMessage) -> Result<(), InstanceHandleError> {
        Ok(self.local_tx.send(input).await?)
    }

    pub async fn current_priorities(&self) -> Result<Vec<PriorityInfo>, InstanceHandleError> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(InstanceMessage::PriorityInfo(tx)).await?;
        Ok(rx.await?)
    }

    pub async fn config(&self) -> Result<Arc<InstanceConfig>, InstanceHandleError> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(InstanceMessage::Config(tx)).await?;
        Ok(rx.await?)
    }
}
