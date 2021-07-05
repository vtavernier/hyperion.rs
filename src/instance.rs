use std::sync::Arc;

use thiserror::Error;
use tokio::sync::broadcast;
use tokio::{select, sync::mpsc};

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
    device: Device,
    receiver: broadcast::Receiver<InputMessage>,
    local_receiver: mpsc::Receiver<InputMessage>,
    muxer: PriorityMuxer,
    core: Core,
    _boblight_server: ServerHandle,
}

impl Instance {
    pub async fn new(global: Global, config: InstanceConfig) -> Result<Self, InstanceError> {
        let device = Device::new(&config.instance.friendly_name, config.device.clone()).await?;
        let receiver = global.subscribe_input().await;
        let (tx, local_receiver) = mpsc::channel(4);

        let muxer = PriorityMuxer::new(global.clone()).await;
        let core = Core::new(&config).await;
        let _boblight_server = servers::bind(
            "Boblight",
            config.boblight_server.clone(),
            global.clone(),
            {
                let instance = Arc::new(config);
                move |tcp, global| {
                    servers::boblight::handle_client(tcp, tx.clone(), instance.clone(), global)
                }
            },
        )
        .await?;

        Ok(Self {
            device,
            receiver,
            local_receiver,
            muxer,
            core,
            _boblight_server,
        })
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
