use std::sync::Arc;

use super::InputMessageData;
use crate::{image::RawImage, models::Color};

#[derive(Debug, Clone)]
pub struct MuxedMessage {
    data: MuxedMessageData,
}

impl MuxedMessage {
    pub fn new(data: MuxedMessageData) -> Self {
        Self { data }
    }

    pub fn data(&self) -> &MuxedMessageData {
        &self.data
    }
}

impl std::ops::Deref for MuxedMessage {
    type Target = MuxedMessageData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Debug, Clone)]
pub enum MuxedMessageData {
    SolidColor {
        priority: i32,
        duration: Option<chrono::Duration>,
        color: Color,
    },
    Image {
        priority: i32,
        duration: Option<chrono::Duration>,
        image: Arc<RawImage>,
    },
    LedColors {
        priority: i32,
        duration: Option<chrono::Duration>,
        led_colors: Arc<Vec<Color>>,
    },
}

impl MuxedMessageData {
    pub fn priority(&self) -> i32 {
        match self {
            MuxedMessageData::SolidColor { priority, .. } => *priority,
            MuxedMessageData::Image { priority, .. } => *priority,
            MuxedMessageData::LedColors { priority, .. } => *priority,
        }
    }

    pub fn color(&self) -> Option<Color> {
        match self {
            MuxedMessageData::SolidColor { color, .. } => Some(*color),
            _ => None,
        }
    }
}

impl From<InputMessageData> for MuxedMessageData {
    fn from(data: InputMessageData) -> Self {
        match data {
            InputMessageData::ClearAll => panic!("ClearAll cannot be muxed"),
            InputMessageData::Clear { .. } => panic!("Clear cannot be muxed"),
            InputMessageData::SolidColor {
                priority,
                duration,
                color,
            } => Self::SolidColor {
                priority,
                duration,
                color,
            },
            InputMessageData::Image {
                priority,
                duration,
                image,
            } => Self::Image {
                priority,
                duration,
                image,
            },
            InputMessageData::LedColors {
                priority,
                duration,
                led_colors,
            } => Self::LedColors {
                priority,
                duration,
                led_colors,
            },
        }
    }
}
