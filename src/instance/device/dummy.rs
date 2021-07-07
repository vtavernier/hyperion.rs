use async_trait::async_trait;

use super::{DeviceError, DeviceImpl};
use crate::models;

pub struct DummyDevice;

impl DummyDevice {
    pub fn new(_config: models::Dummy) -> Self {
        Self
    }
}

#[async_trait]
impl DeviceImpl for DummyDevice {
    async fn set_led_data(&mut self, led_data: &[models::Color]) -> Result<(), DeviceError> {
        // Write to log when we get new data
        for (i, led) in led_data.iter().enumerate() {
            info!(
                led = %format_args!("{:3}", i),
                red = %format_args!("{:3}", led.red),
                green = %format_args!("{:3}", led.green),
                blue = %format_args!("{:3}", led.blue),
            );
        }

        Ok(())
    }

    async fn update(&mut self) -> Result<(), super::DeviceError> {
        // No regular updates
        futures::future::pending().await
    }
}
