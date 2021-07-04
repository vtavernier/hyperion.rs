use std::sync::Arc;

use super::{InputMessageData, Message};
use crate::{component::ComponentName, image::RawImage, models::Color};

#[derive(Debug, Clone)]
pub struct MuxedMessage {
    source_id: usize,
    component: ComponentName,
    data: MuxedMessageData,
}

impl Message for MuxedMessage {
    type Data = MuxedMessageData;

    fn new(source_id: usize, component: ComponentName, data: Self::Data) -> Self {
        Self {
            source_id,
            component,
            data,
        }
    }

    fn source_id(&self) -> usize {
        self.source_id
    }

    fn component(&self) -> ComponentName {
        self.component
    }

    fn data(&self) -> &Self::Data {
        &self.data
    }

    fn unregister_source(global: &mut super::GlobalData, input_source: &super::InputSource<Self>) {
        global.unregister_muxed_source(input_source);
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
            InputMessageData::PrioritiesRequest { .. } => {
                panic!("PrioritiesRequest cannot be muxed")
            }
        }
    }
}
