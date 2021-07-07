use std::convert::TryFrom;
use std::net::SocketAddr;
use std::sync::Arc;

use thiserror::Error;

use super::types::i32_to_duration;

use crate::{
    component::ComponentName,
    global::{Global, InputMessage, InputMessageData, InputSourceHandle, InputSourceName},
    image::{RawImage, RawImageError},
    models::Color,
};

/// Schema definitions as Serde serializable structures and enums
pub mod message;

#[derive(Debug, Error)]
pub enum FlatApiError {
    #[error("error broadcasting update: {0}")]
    Broadcast(#[from] tokio::sync::broadcast::error::SendError<InputMessage>),
    #[error("source not registered")]
    Unregistered,
    #[error("invalid priority for registration, should be in [100, 200), got {0}")]
    InvalidPriority(i32),
    #[error("unknown command")]
    UnknownCommand,
    #[error("error decoding image: {0}")]
    RawImageError(#[from] RawImageError),
}

async fn handle_register(
    peer_addr: SocketAddr,
    register: message::Register<'_>,
    source: &mut Option<InputSourceHandle<InputMessage>>,
    global: &Global,
) -> Result<(), FlatApiError> {
    let priority = register.priority();

    if priority < 100 || priority >= 200 {
        return Err(FlatApiError::InvalidPriority(priority));
    } else {
        // unwrap: we checked the priority value before
        *source = Some(
            global
                .register_input_source(
                    InputSourceName::FlatBuffers {
                        peer_addr,
                        origin: register.origin().to_owned(),
                    },
                    Some(priority),
                )
                .await
                .unwrap(),
        );
    }

    Ok(())
}

pub async fn handle_request(
    peer_addr: SocketAddr,
    request: message::Request<'_>,
    source: &mut Option<InputSourceHandle<InputMessage>>,
    global: &Global,
) -> Result<(), FlatApiError> {
    if let Some(handle) = source.as_ref() {
        // unwrap: we set a priority when we got the register call
        let priority = handle.priority().unwrap();

        if let Some(clear) = request.command_as_clear() {
            // Update state
            if clear.priority() < 0 {
                handle.send(ComponentName::FlatbufServer, InputMessageData::ClearAll)?;
            } else {
                handle.send(
                    ComponentName::FlatbufServer,
                    InputMessageData::Clear {
                        priority: clear.priority(),
                    },
                )?;
            }
        } else if let Some(color) = request.command_as_color() {
            let rgb = color.data();
            let rgb = (
                (rgb & 0x000_000FF) as u8,
                ((rgb & 0x0000_FF00) >> 8) as u8,
                ((rgb & 0x00FF_0000) >> 16) as u8,
            );

            // Update state
            handle.send(
                ComponentName::FlatbufServer,
                InputMessageData::SolidColor {
                    // TODO
                    priority: 0,
                    duration: i32_to_duration(Some(color.duration())),
                    color: Color::from_components(rgb),
                },
            )?;
        } else if let Some(image) = request.command_as_image() {
            // Get raw image
            let data = image
                .data_as_raw_image()
                .ok_or_else(|| RawImageError::RawImageMissing)?;

            // Extract fields
            let duration = image.duration();
            let width = data.width();
            let height = data.height();
            let data = data.data().ok_or_else(|| RawImageError::RawImageMissing)?;

            // Parse message
            let width = u32::try_from(width).map_err(|_| RawImageError::InvalidWidth)?;
            let height = u32::try_from(height).map_err(|_| RawImageError::InvalidHeight)?;
            let raw_image = RawImage::try_from((data.to_vec(), width, height))?;

            // Update state
            handle.send(
                ComponentName::FlatbufServer,
                InputMessageData::Image {
                    priority,
                    duration: i32_to_duration(Some(duration)),
                    image: Arc::new(raw_image),
                },
            )?;
        } else if let Some(register) = request.command_as_register() {
            return handle_register(peer_addr, register, source, global).await;
        } else {
            return Err(FlatApiError::UnknownCommand);
        }
    } else {
        if let Some(register) = request.command_as_register() {
            return handle_register(peer_addr, register, source, global).await;
        } else {
            return Err(FlatApiError::Unregistered);
        }
    };

    Ok(())
}
