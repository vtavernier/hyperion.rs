//! Definition of the StateUpdate type

use crate::color;
use crate::image;

/// State update data
#[derive(Clone)]
pub enum StateUpdate {
    /// Clear all devices
    Clear,
    /// Set all devices to a given color
    SolidColor {
        /// Color to apply to the devices
        color: color::ColorPoint,
    },
    /// Use given image to set colors
    Image(image::RawImage),
    /// Per-LED color data
    LedData(Vec<color::ColorPoint>),
}

impl std::fmt::Debug for StateUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StateUpdate::Clear => write!(f, "Clear"),
            StateUpdate::SolidColor { color } => write!(f, "SolidColor {{ color: {:?} }}", color),
            StateUpdate::Image(image) => {
                let (width, height) = image.get_dimensions();
                write!(f, "Image({}x{} image)", width, height)
            }
            StateUpdate::LedData(led_data) => write!(f, "LedData({} LEDs)", led_data.len()),
        }
    }
}

impl StateUpdate {
    /// Create a Clear StateUpdate
    pub fn clear() -> Self {
        Self::Clear
    }

    /// Create a SolidColor StateUpdate
    ///
    /// # Parameters
    ///
    /// * `color`: color to apply to the devices
    pub fn solid(color: color::ColorPoint) -> Self {
        Self::SolidColor { color }
    }

    /// Create an Image StateUpdate
    ///
    /// # Parameters
    ///
    /// * `image`: raw image to extract colors from
    pub fn image(image: image::RawImage) -> Self {
        Self::Image(image)
    }

    /// Create a LedData StateUpdate
    ///
    /// # Parameters
    ///
    /// * `led_data`: LED data
    pub fn led_data(led_data: Vec<color::ColorPoint>) -> Self {
        Self::LedData(led_data)
    }
}