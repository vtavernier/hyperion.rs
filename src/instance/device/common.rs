use std::time::Instant;

use async_trait::async_trait;

use super::{DeviceError, DeviceImpl};
use crate::models::{self, DeviceConfig};

#[async_trait]
pub trait WritingDevice: Send {
    type Config: DeviceConfig;

    async fn set_let_data(
        &mut self,
        config: &Self::Config,
        led_data: &[models::Color],
    ) -> Result<(), DeviceError>;
    async fn write(&mut self) -> Result<(), DeviceError>;
}

pub struct Rewriter<D: WritingDevice> {
    inner: D,
    config: D::Config,
    last_write_time: Option<Instant>,
}

impl<D: WritingDevice> Rewriter<D> {
    pub fn new(inner: D, config: D::Config) -> Self {
        Self {
            inner,
            config,
            last_write_time: None,
        }
    }

    async fn write(&mut self) -> Result<(), DeviceError> {
        self.inner.write().await?;
        self.last_write_time = Some(Instant::now());
        Ok(())
    }
}

#[async_trait]
impl<D: WritingDevice> DeviceImpl for Rewriter<D> {
    async fn set_led_data(&mut self, led_data: &[models::Color]) -> Result<(), DeviceError> {
        self.inner.set_let_data(&self.config, led_data).await?;

        // Immediately write to device
        self.write().await?;

        Ok(())
    }

    async fn update(&mut self) -> Result<(), DeviceError> {
        if let Some(rewrite_time) = self.config.rewrite_time() {
            let now = Instant::now();
            let next_rewrite_time = self
                .last_write_time
                .map(|lwt| lwt + rewrite_time)
                .unwrap_or(now);

            // Wait until the next rewrite cycle if necessary
            if next_rewrite_time > now {
                tokio::time::sleep_until(next_rewrite_time.into()).await;
            }

            // Write to device
            self.write().await?;

            Ok(())
        } else {
            futures::future::pending().await
        }
    }
}
