use std::convert::TryFrom;
use std::sync::Arc;

use thiserror::Error;
use validator::Validate;

use crate::{
    component::ComponentName,
    global::{Global, InputMessage, InputMessageData, InputSourceHandle},
    image::{RawImage, RawImageError},
    instance::{InstanceHandle, InstanceHandleError},
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
    #[error("error accessing the current instance: {0}")]
    Instance(#[from] InstanceHandleError),
}

/// A client connected to the JSON endpoint
pub struct ClientConnection {
    source: InputSourceHandle<InputMessage>,
    current_instance: Option<i32>,
}

impl ClientConnection {
    pub fn new(source: InputSourceHandle<InputMessage>) -> Self {
        Self {
            source,
            current_instance: None,
        }
    }

    async fn current_instance(&mut self, global: &Global) -> Option<InstanceHandle> {
        if let Some(current_instance) = self.current_instance {
            if let Some(instance) = global.get_instance(current_instance).await {
                return Some(instance);
            } else {
                // Instance id now invalid, reset
                self.current_instance = None;
            }
        }

        if let Some((id, inst)) = global.default_instance().await {
            self.set_current_instance(id);
            return Some(inst);
        }

        None
    }

    fn set_current_instance(&mut self, id: i32) {
        debug!("{}: switch to instance {}", &self.source.name(), id);
        self.current_instance = Some(id);
    }

    #[instrument(skip(request, global))]
    pub async fn handle_request(
        &mut self,
        request: HyperionMessage,
        global: &Global,
    ) -> Result<Option<HyperionResponse>, JsonApiError> {
        request.validate()?;

        match request.command {
            HyperionCommand::ClearAll => {
                // Update state
                self.source
                    .send(ComponentName::All, InputMessageData::ClearAll)?;
            }

            HyperionCommand::Clear(message::Clear { priority }) => {
                // Update state
                self.source
                    .send(ComponentName::All, InputMessageData::Clear { priority })?;
            }

            HyperionCommand::Color(message::Color {
                priority,
                duration,
                color,
                origin: _,
            }) => {
                // TODO: Handle origin field

                // Update state
                self.source.send(
                    ComponentName::Color,
                    InputMessageData::SolidColor {
                        priority,
                        duration: duration.map(|ms| chrono::Duration::milliseconds(ms as _)),
                        color,
                    },
                )?;
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

                self.source.send(
                    ComponentName::Image,
                    InputMessageData::Image {
                        priority,
                        duration: duration.map(|ms| chrono::Duration::milliseconds(ms as _)),
                        image: Arc::new(raw_image),
                    },
                )?;
            }

            HyperionCommand::ServerInfo(message::ServerInfoRequest { subscribe: _ }) => {
                // TODO: Handle subscribe field

                let (adjustments, priorities) =
                    if let Some(handle) = self.current_instance(global).await {
                        (
                            handle
                                .config()
                                .await?
                                .color
                                .channel_adjustment
                                .iter()
                                .map(|adj| message::ChannelAdjustment::from(adj.clone()))
                                .collect(),
                            handle.current_priorities().await?,
                        )
                    } else {
                        Default::default()
                    };

                // Just answer the serverinfo request, no need to update state
                return Ok(Some(
                    global
                        .read_config(|config| {
                            let instances = config
                                .instances
                                .iter()
                                .map(|instance_config| (&instance_config.1.instance).into())
                                .collect();

                            HyperionResponse::server_info(
                                request.tan,
                                priorities,
                                adjustments,
                                // TODO: Fill effects
                                vec![],
                                instances,
                            )
                        })
                        .await,
                ));
            }

            HyperionCommand::Authorize(message::Authorize { subcommand, .. }) => match subcommand {
                message::AuthorizeCommand::AdminRequired => {
                    // TODO: Perform actual authentication flow
                    return Ok(Some(HyperionResponse::admin_required(request.tan, false)));
                }
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

            HyperionCommand::Instance(message::Instance {
                subcommand: message::InstanceCommand::SwitchTo,
                instance: Some(id),
                ..
            }) => {
                if global.get_instance(id).await.is_some() {
                    self.set_current_instance(id);
                    return Ok(Some(HyperionResponse::switch_to(request.tan, Some(id))));
                } else {
                    // Note: it's an "Ok" but should be an Err. Find out how to represent errors
                    // better
                    return Ok(Some(HyperionResponse::switch_to(request.tan, None)));
                }
            }

            _ => return Err(JsonApiError::NotImplemented),
        };

        Ok(None)
    }
}

impl std::fmt::Debug for ClientConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientConnection")
            .field("source", &format!("{}", &*self.source))
            .finish()
    }
}
