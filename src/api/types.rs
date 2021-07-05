use serde::Serialize;

use crate::{
    component::ComponentName,
    global::{InputMessageData, Message},
    models::Color,
};

fn not_positive(x: &i64) -> bool {
    !(*x > 0)
}

fn color_to_hsl(color: Color) -> palette::Hsl {
    let (r, g, b) = color.into_components();
    palette::Hsl::from(palette::LinSrgb::new(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
    ))
}

#[derive(Debug, Serialize)]
pub struct PriorityInfo {
    pub priority: i32,
    #[serde(skip_serializing_if = "not_positive")]
    pub duration_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub component_id: ComponentName,
    pub origin: String,
    pub active: bool,
    pub visible: bool,
    #[serde(rename = "RGB", skip_serializing_if = "Option::is_none")]
    pub value: Option<LedColor>,
}

impl PriorityInfo {
    pub fn new(
        msg: &crate::global::InputMessage,
        origin: String,
        expires: Option<std::time::Instant>,
        visible: bool,
    ) -> Self {
        let duration_ms = expires
            .and_then(|when| {
                let now = std::time::Instant::now();

                if when > now {
                    chrono::Duration::from_std(when - now).ok()
                } else {
                    Some(chrono::Duration::zero())
                }
            })
            .map(|d| d.num_milliseconds() as i64)
            .unwrap_or(-1);
        let active = duration_ms >= -1;

        match msg.data() {
            InputMessageData::SolidColor {
                priority, color, ..
            } => Self {
                priority: *priority,
                duration_ms,
                owner: None,
                component_id: msg.component(),
                origin,
                active,
                visible,
                value: Some(color.into()),
            },
            InputMessageData::Image { priority, .. }
            | InputMessageData::LedColors { priority, .. } => Self {
                priority: *priority,
                duration_ms,
                owner: None,
                component_id: msg.component(),
                origin,
                active,
                visible,
                value: None,
            },
            InputMessageData::Clear { .. } | InputMessageData::ClearAll { .. } => {
                panic!("cannot create PriorityInfo for InputMessage")
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct LedColor {
    pub rgb: [u8; 3],
    pub hsl: (u16, f32, f32),
}

impl From<&Color> for LedColor {
    fn from(c: &Color) -> Self {
        let hsl = color_to_hsl(*c);

        Self {
            rgb: [c.red, c.green, c.blue],
            hsl: (
                (hsl.hue.to_positive_degrees() * 100.) as u16,
                hsl.saturation,
                hsl.lightness,
            ),
        }
    }
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
