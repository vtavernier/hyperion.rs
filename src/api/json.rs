use std::convert::TryFrom;
use std::sync::Arc;

use thiserror::Error;
use validator::Validate;

use crate::{
    global::{Global, InputMessage, InputMessageData, InputSourceHandle},
    image::{RawImage, RawImageError},
};

/// Schema definitions as Serde serializable structures and enums
pub mod message;
use message::{HyperionCommand, HyperionMessage, HyperionResponse};

#[derive(Debug, Error)]
pub enum JsonApiError {
    #[error("error broadcasting update: {0}")]
    Broadcast(#[from] tokio::sync::broadcast::error::SendError<InputMessage>),
    #[error("request not implemented")]
    NotImplemented,
    #[error("error decoding image")]
    Image(#[from] RawImageError),
    #[error("error validating request: {0}")]
    Validation(#[from] validator::ValidationErrors),
    #[error("error receiving system response: {0}")]
    Recv(#[from] tokio::sync::oneshot::error::RecvError),
}

/// A client connected to the JSON endpoint
pub struct ClientConnection {
    source: InputSourceHandle<InputMessage>,
}

impl ClientConnection {
    pub fn new(source: InputSourceHandle<InputMessage>) -> Self {
        Self { source }
    }

    pub async fn handle_request(
        &self,
        request: HyperionMessage,
        global: &Global,
    ) -> Result<Option<HyperionResponse>, JsonApiError> {
        request.validate()?;

        match request.command {
            HyperionCommand::ClearAll => {
                // Update state
                self.source.send(InputMessageData::ClearAll)?;
            }

            HyperionCommand::Clear(message::Clear { priority }) => {
                // Update state
                self.source.send(InputMessageData::Clear { priority })?;
            }

            HyperionCommand::Color(message::Color {
                priority,
                duration,
                color,
                origin: _,
            }) => {
                // TODO: Handle origin field

                // Update state
                self.source.send(InputMessageData::SolidColor {
                    priority,
                    duration: duration.map(|ms| chrono::Duration::milliseconds(ms as _)),
                    color,
                })?;
            }

            HyperionCommand::Image(message::Image {
                priority,
                duration,
                imagewidth,
                imageheight,
                imagedata,
                origin: _,
                format: _,
                scale: _,
                name: _,
            }) => {
                // TODO: Handle origin, format, scale, name fields

                let raw_image = RawImage::try_from((imagedata, imagewidth, imageheight))?;

                self.source.send(InputMessageData::Image {
                    priority,
                    duration: duration.map(|ms| chrono::Duration::milliseconds(ms as _)),
                    image: Arc::new(raw_image),
                })?;
            }

            HyperionCommand::ServerInfo(message::ServerInfoRequest { subscribe: _ }) => {
                // TODO: Handle subscribe field

                // Request priority information
                let (sender, receiver) = tokio::sync::oneshot::channel();
                self.source.send(InputMessageData::PrioritiesRequest {
                    response: Arc::new(std::sync::Mutex::new(Some(sender))),
                })?;

                // Receive priority information
                let priorities = receiver
                    .await?
                    .into_iter()
                    .map(message::PriorityInfo::from)
                    .collect();

                // Just answer the serverinfo request, no need to update state
                return Ok(Some(HyperionResponse::server_info(
                    request.tan,
                    vec![],
                    priorities,
                    global
                        .read_config(|config| {
                            config
                                .instances
                                .iter()
                                .map(|instance_config| (&instance_config.1.instance).into())
                                .collect()
                        })
                        .await,
                )));
            }

            HyperionCommand::Authorize(message::Authorize { subcommand, .. }) => match subcommand {
                message::AuthorizeCommand::TokenRequired => {
                    // TODO: Perform actual authentication flow
                    return Ok(Some(HyperionResponse::token_required(request.tan, false)));
                }
                _ => {
                    return Err(JsonApiError::NotImplemented);
                }
            },

            HyperionCommand::SysInfo => {
                return Ok(Some(HyperionResponse::sys_info(
                    request.tan,
                    global.read_config(|config| config.uuid()).await,
                )));
            }

            _ => return Err(JsonApiError::NotImplemented),
        };

        Ok(None)
    }
}
