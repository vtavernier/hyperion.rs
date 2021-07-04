use std::convert::TryFrom;
use std::sync::Arc;

use thiserror::Error;

use super::types::i32_to_duration;

use crate::{
    component::ComponentName,
    global::{InputMessage, InputMessageData, InputSourceHandle},
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
}

pub fn handle_request(
    request: HyperionRequest,
    source: &InputSourceHandle<InputMessage>,
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

            // Update state
            source.send(
                ComponentName::ProtoServer,
                InputMessageData::SolidColor {
                    priority: color_request.priority,
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

            // Update state
            source.send(
                ComponentName::ProtoServer,
                InputMessageData::Image {
                    priority: image_request.priority,
                    duration: i32_to_duration(image_request.duration),
                    image: Arc::new(raw_image),
                },
            )?;
        }
    }

    Ok(())
}
