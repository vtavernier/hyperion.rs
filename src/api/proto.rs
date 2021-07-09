use std::sync::Arc;
use std::{convert::TryFrom, net::SocketAddr};

use thiserror::Error;

use super::types::i32_to_duration;

use crate::{
    component::ComponentName,
    global::{InputMessage, InputMessageData, InputSourceHandle, PriorityGuard},
    image::{RawImage, RawImageError},
    models::Color,
};

/// Schema definitions as Serde serializable structures and enums
pub mod message;
use message::HyperionRequest;

#[derive(Debug, Error)]
pub enum ProtoApiError {
    #[error("error decoding image: {0}")]
    RawImageError(#[from] RawImageError),
    #[error("error broadcasting update: {0}")]
    Broadcast(#[from] tokio::sync::broadcast::error::SendError<InputMessage>),
    #[error("missing command data in protobuf frame")]
    MissingCommand,
    #[error("the priority {0} is not in the valid range between 100 and 199")]
    InvalidPriority(i32),
}

fn validate_priority(
    priority: i32,
    source: &InputSourceHandle<InputMessage>,
    priority_guard: &mut PriorityGuard,
) -> Result<i32, ProtoApiError> {
    if priority < 100 || priority >= 200 {
        return Err(ProtoApiError::InvalidPriority(priority));
    }

    // Re-creating the priority guard drops the old value, thus clearing the previous priority
    *priority_guard = PriorityGuard::new_broadcast(source);

    Ok(priority)
}

#[instrument(skip(request, source, priority_guard))]
pub fn handle_request(
    peer_addr: SocketAddr,
    request: HyperionRequest,
    source: &InputSourceHandle<InputMessage>,
    priority_guard: &mut PriorityGuard,
) -> Result<(), ProtoApiError> {
    match request.command() {
        message::hyperion_request::Command::Clearall => {
            // Update state
            source.send(ComponentName::ProtoServer, InputMessageData::ClearAll)?;
        }

        message::hyperion_request::Command::Clear => {
            let clear_request = request
                .clear_request
                .ok_or_else(|| ProtoApiError::MissingCommand)?;

            // Update state
            source.send(
                ComponentName::ProtoServer,
                InputMessageData::Clear {
                    priority: clear_request.priority,
                },
            )?;
        }

        message::hyperion_request::Command::Color => {
            let color_request = request
                .color_request
                .ok_or_else(|| ProtoApiError::MissingCommand)?;

            let color = color_request.rgb_color;
            let color = (
                (color & 0x000_000FF) as u8,
                ((color & 0x0000_FF00) >> 8) as u8,
                ((color & 0x00FF_0000) >> 16) as u8,
            );

            let priority = validate_priority(color_request.priority, source, priority_guard)?;

            // Update state
            source.send(
                ComponentName::ProtoServer,
                InputMessageData::SolidColor {
                    priority,
                    duration: i32_to_duration(color_request.duration),
                    color: Color::from_components(color),
                },
            )?;
        }

        message::hyperion_request::Command::Image => {
            let image_request = request
                .image_request
                .ok_or_else(|| ProtoApiError::MissingCommand)?;

            let width =
                u32::try_from(image_request.imagewidth).map_err(|_| RawImageError::InvalidWidth)?;
            let height = u32::try_from(image_request.imageheight)
                .map_err(|_| RawImageError::InvalidHeight)?;
            let raw_image = RawImage::try_from((image_request.imagedata.to_vec(), width, height))?;

            let priority = validate_priority(image_request.priority, source, priority_guard)?;

            // Update state
            source.send(
                ComponentName::ProtoServer,
                InputMessageData::Image {
                    priority,
                    duration: i32_to_duration(image_request.duration),
                    image: Arc::new(raw_image),
                },
            )?;
        }
    }

    Ok(())
}
