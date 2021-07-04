use std::sync::{Arc, Mutex};

use super::Message;
use crate::{
    api::json::message::PriorityInfo, component::ComponentName, image::RawImage, models::Color,
};

#[derive(Debug, Clone)]
pub struct InputMessage {
    source_id: usize,
    component: ComponentName,
    data: InputMessageData,
}

impl Message for InputMessage {
    type Data = InputMessageData;

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
        global.unregister_input_source(input_source);
    }
}

#[derive(Debug, Clone)]
pub enum InputMessageData {
    ClearAll,
    Clear {
        priority: i32,
    },
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
    PrioritiesRequest {
        response: Arc<Mutex<Option<tokio::sync::oneshot::Sender<Vec<PriorityInfo>>>>>,
    },
}

impl InputMessageData {
    pub fn priority(&self) -> Option<i32> {
        match self {
            InputMessageData::ClearAll => None,
            InputMessageData::Clear { priority } => Some(*priority),
            InputMessageData::SolidColor { priority, .. } => Some(*priority),
            InputMessageData::Image { priority, .. } => Some(*priority),
            InputMessageData::PrioritiesRequest { .. } => None,
        }
    }

    pub fn duration(&self) -> Option<chrono::Duration> {
        match self {
            InputMessageData::ClearAll => None,
            InputMessageData::Clear { .. } => None,
            InputMessageData::SolidColor { duration, .. } => *duration,
            InputMessageData::Image { duration, .. } => *duration,
            InputMessageData::PrioritiesRequest { .. } => None,
        }
    }
}
