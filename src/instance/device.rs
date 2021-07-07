use async_trait::async_trait;
use thiserror::Error;

use crate::models::{self, DeviceConfig};

mod common;

// Device implementation modules

mod dummy;
mod ws2812spi;

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("device not supported: {0}")]
    NotSupported(&'static str),
    #[error("i/o error: {0}")]
    FuturesIo(#[from] futures_io::Error),
}

#[async_trait]
trait DeviceImpl: Send {
    /// Set the device implementation's view of the LED data to the given values
    ///
    /// # Panics
    ///
    /// Implementations are allowed to panic if led_data.len() != hardware_led_count. The [Device]
    /// wrapper is responsible for ensuring the given slice is the right size.
    async fn set_led_data(&mut self, led_data: &[models::Color]) -> Result<(), DeviceError>;

    /// Update the device implementation's temporal data. For devices that require regular rewrites
    /// (regardless of actual changes in the LED data), this should return a future that performs
    /// the required work.
    async fn update(&mut self) -> Result<(), DeviceError>;
}

pub struct Device {
    name: String,
    inner: Box<dyn DeviceImpl>,
    led_data: Vec<models::Color>,
    notified_inconsistent_led_data: bool,
}

impl Device {
    fn build_inner(config: models::Device) -> Result<Box<dyn DeviceImpl>, DeviceError> {
        let inner: Box<dyn DeviceImpl>;
        match config {
            models::Device::Dummy(dummy) => {
                inner = Box::new(dummy::DummyDevice::new(dummy)?);
            }
            models::Device::Ws2812Spi(ws2812spi) => {
                inner = Box::new(ws2812spi::Ws2812SpiDevice::new(ws2812spi)?);
            }
            other => {
                return Err(DeviceError::NotSupported(other.into()));
            }
        }

        Ok(inner)
    }

    #[instrument(skip(config))]
    pub async fn new(name: &str, config: models::Device) -> Result<Self, DeviceError> {
        let led_count = config.hardware_led_count();
        let inner = Self::build_inner(config)?;

        Ok(Self {
            name: name.to_owned(),
            inner,
            led_data: vec![Default::default(); led_count],
            notified_inconsistent_led_data: false,
        })
    }

    #[instrument(skip(led_data))]
    pub async fn set_led_data(&mut self, led_data: &[models::Color]) -> Result<(), DeviceError> {
        // Store the LED data for updates
        let led_count = led_data.len();
        let hw_led_count = self.led_data.len();

        if led_count == hw_led_count {
            self.led_data.copy_from_slice(led_data);
            self.notified_inconsistent_led_data = false;
        } else if led_count > hw_led_count {
            // Too much data in led_data
            // Take only the slice that fits
            self.led_data.copy_from_slice(&led_data[..hw_led_count]);

            if !self.notified_inconsistent_led_data {
                self.notified_inconsistent_led_data = true;
                warn!(
                    "too much LED data for device: {} extra",
                    led_count - hw_led_count
                );
            }
        } else {
            // Not enough data
            // Take the given data
            self.led_data[..led_count].copy_from_slice(led_data);
            // And pad with zeros
            self.led_data[led_count..].fill(Default::default());

            if !self.notified_inconsistent_led_data {
                self.notified_inconsistent_led_data = true;
                warn!(
                    "not enough LED data for device: {} missing",
                    hw_led_count - led_count
                );
            }
        }

        // Notify device of new write: some devices write immediately
        self.inner.set_led_data(&self.led_data).await
    }

    #[instrument]
    pub async fn update(&mut self) -> Result<(), DeviceError> {
        Ok(self.inner.update().await?)
    }
}

impl std::fmt::Debug for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Device").field("name", &self.name).finish()
    }
}
