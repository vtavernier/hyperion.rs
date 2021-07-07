use std::sync::Arc;

use thiserror::Error;

use crate::{
    global::{InputMessage, InputMessageData, InputSourceHandle, Message},
    instance::{InstanceHandle, InstanceHandleError},
    models::Color,
};

pub mod message;
use message::{BoblightRequest, BoblightResponse};

#[derive(Debug, Error)]
pub enum BoblightApiError {
    #[error("error broadcasting update: {0}")]
    Broadcast(#[from] tokio::sync::mpsc::error::SendError<InputMessage>),
    #[error("missing command data in protobuf frame")]
    MissingCommand,
    #[error("invalid instance")]
    InvalidInstance(#[from] InstanceHandleError),
}

pub struct ClientConnection {
    handle: InputSourceHandle<InputMessage>,
    led_colors: Vec<Color>,
    priority: i32,
    instance: InstanceHandle,
}

impl ClientConnection {
    pub fn new(
        handle: InputSourceHandle<InputMessage>,
        led_count: usize,
        instance: InstanceHandle,
    ) -> Self {
        Self {
            handle,
            led_colors: vec![Color::default(); led_count],
            priority: 128,
            instance,
        }
    }

    fn set_priority(&mut self, priority: i32) {
        if priority < 128 || priority >= 254 {
            // TODO: Find first available priority
            self.priority = 128;
        } else {
            self.priority = priority;
        }
    }

    async fn sync(&self) -> Result<(), BoblightApiError> {
        Ok(self
            .instance
            .send(InputMessage::new(
                self.handle.id(),
                crate::component::ComponentName::BoblightServer,
                InputMessageData::LedColors {
                    priority: self.priority,
                    duration: None,
                    led_colors: Arc::new(self.led_colors.clone()),
                },
            ))
            .await?)
    }

    pub async fn handle_request(
        &mut self,
        request: BoblightRequest,
    ) -> Result<Option<BoblightResponse>, BoblightApiError> {
        match request {
            BoblightRequest::Hello => Ok(Some(BoblightResponse::Hello)),
            BoblightRequest::Ping => Ok(Some(BoblightResponse::Ping)),
            BoblightRequest::Get(get) => match get {
                message::GetArg::Version => Ok(Some(BoblightResponse::Version)),
                message::GetArg::Lights => Ok(Some(BoblightResponse::Lights {
                    leds: self.instance.config().await?.leds.leds.clone(),
                })),
            },
            BoblightRequest::Set(set) => {
                match set {
                    message::SetArg::Light(message::LightParam { index, data }) => match data {
                        message::LightParamData::Color(color) => {
                            if let Some(color_mut) = self.led_colors.get_mut(index) {
                                *color_mut = color;

                                if index == self.led_colors.len() - 1 {
                                    self.sync().await?;
                                }
                            }
                        }
                        _ => {}
                    },
                    message::SetArg::Priority(priority) => {
                        self.set_priority(priority);
                    }
                }

                Ok(None)
            }
            BoblightRequest::Sync => {
                self.sync().await?;

                Ok(None)
            }
        }
    }
}
