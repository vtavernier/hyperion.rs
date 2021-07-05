use std::sync::Arc;

use thiserror::Error;
use tokio::sync::broadcast;
use tokio::{select, sync::mpsc};

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
    device: InstanceDevice,
    receiver: broadcast::Receiver<InputMessage>,
    local_receiver: mpsc::Receiver<InputMessage>,
    muxer: PriorityMuxer,
    core: Core,
    _boblight_server: Option<Result<ServerHandle, std::io::Error>>,
}

impl Instance {
    pub async fn new(global: Global, config: InstanceConfig) -> Self {
        let device: InstanceDevice =
            Device::new(&config.instance.friendly_name, config.device.clone())
                .await
                .into();

        if let Err(error) = &device.inner {
            error!(
                "Initializing instance {} `{}` failed: {}",
                config.instance.id, config.instance.friendly_name, error
            );
        }

        let receiver = global.subscribe_input().await;
        let (tx, local_receiver) = mpsc::channel(4);

        let muxer = PriorityMuxer::new(global.clone()).await;
        let core = Core::new(&config).await;

        let config = Arc::new(config);
        let _boblight_server = if config.boblight_server.enable {
            let server_handle = servers::bind(
                "Boblight",
                config.boblight_server.clone(),
                global.clone(),
                {
                    let instance = config.clone();
                    move |tcp, global| {
                        servers::boblight::handle_client(tcp, tx.clone(), instance.clone(), global)
                    }
                },
            )
            .await;

            if let Err(error) = &server_handle {
                error!(
                    "Cannot start Boblight server for instance {} `{}` failed: {}",
                    config.instance.id, config.instance.friendly_name, error
                );
            }

            Some(server_handle)
        } else {
            None
        };

        Self {
            device,
            receiver,
            local_receiver,
            muxer,
            core,
            _boblight_server,
        }
    }

    async fn on_input_message(&mut self, message: InputMessage) {
        if let Some(message) = self.muxer.handle_message(message).await {
            // The message triggered a muxing update
            self.core.handle_message(message);
        }
    }

    pub async fn run(mut self) -> Result<(), InstanceError> {
        loop {
            select! {
                _ = self.device.update() => {
                    // Device update completed
                    // TODO: Handle device update errors
                },
                message = self.receiver.recv() => {
                    match message {
                        Ok(message) => {
                            self.on_input_message(message).await;
                        },
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            // No more input messages
                            return Ok(());
                        },
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!("skipped {} input messages", skipped);
                        },
                    }
                },
                message = self.local_receiver.recv() => {
                    if let Some(message) = message {
                        self.on_input_message(message).await;
                    } else {
                        break Ok(());
                    }
                },
                message = self.muxer.update() => {
                    // Muxer update completed
                    if let Some(message) = message {
                        self.core.handle_message(message);
                    }
                },
                led_data = self.core.update() => {
                    // LED data changed
                    self.device.set_led_data(led_data).await?;
                },
            }
        }
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
