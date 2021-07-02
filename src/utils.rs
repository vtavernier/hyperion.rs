use crate::models::{Color, Color16};

pub fn color_to8(color: Color16) -> Color {
    let (r, g, b) = color.into_components();
    Color::from_components(((r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8))
}

pub fn color_to16(color: Color) -> Color16 {
    let (r, g, b) = color.into_components();
    Color16::from_components(((r as u16) << 8, (g as u16) << 8, (b as u16) << 8))
}

pub fn i32_to_duration(d: Option<i32>) -> Option<chrono::Duration> {
    if let Some(d) = d {
        if d <= 0 {
            None
        } else {
            Some(chrono::Duration::milliseconds(d as _))
        }
    } else {
        None
    }
}
