use async_trait::async_trait;

use super::{common::*, DeviceError};
use crate::models;

pub type DummyDevice = Rewriter<DummyDeviceImpl>;

pub struct DummyDeviceImpl {
    leds: Vec<models::Color>,
}

#[async_trait]
impl WritingDevice for DummyDeviceImpl {
    type Config = models::Dummy;

    fn new(config: &Self::Config) -> Result<Self, DeviceError> {
        Ok(Self {
            leds: vec![Default::default(); config.hardware_led_count as _],
        })
    }

    async fn set_let_data(
        &mut self,
        _config: &Self::Config,
        led_data: &[models::Color],
    ) -> Result<(), DeviceError> {
        self.leds.copy_from_slice(led_data);
        Ok(())
    }

    async fn write(&mut self) -> Result<(), DeviceError> {
        // Write to log when we get new data
        for (i, led) in self.leds.iter().enumerate() {
            info!(
                led = %format_args!("{:3}", i),
                red = %format_args!("{:3}", led.red),
                green = %format_args!("{:3}", led.green),
                blue = %format_args!("{:3}", led.blue),
            );
        }

        Ok(())
    }
}
