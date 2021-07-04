//! Component system definitions

use parse_display::Display;
use serde::{Deserialize, Serialize};

#[derive(Display, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentName {
    #[display("Hyperion")]
    All,
    #[display("Smoothing")]
    Smoothing,
    #[display("Blackborder detector")]
    BlackBorder,
    #[display("Json/Proto forwarder")]
    Forwarder,
    #[display("Boblight server")]
    BoblightServer,
    #[display("Framegrabber")]
    Grabber,
    #[display("V4L capture device")]
    V4L,
    #[display("Solid color")]
    Color,
    #[display("Effect")]
    Effect,
    #[display("Image")]
    Image,
    #[display("LED device")]
    LedDevice,
    #[display("Image Receiver")]
    FlatbufServer,
    #[display("Proto Server")]
    ProtoServer,
}
