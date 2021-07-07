use std::time::{Duration, Instant};

use async_trait::async_trait;
use spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};

use super::{DeviceError, DeviceImpl};
use crate::models;

// TODO: Support invert
// TODO: Support latch_time

pub struct Ws2812SpiDevice {
    config: models::Ws2812Spi,
    dev: Spidev,
    buf: Vec<u8>,
    last_write_time: Option<Instant>,
}

const SPI_BYTES_PER_LED: usize = 3 * SPI_BYTES_PER_COLOUR;
const SPI_BYTES_PER_COLOUR: usize = 4;
const SPI_FRAME_END_LATCH_BYTES: usize = 116;
const BITPAIR_TO_BYTE: [u8; 4] = [0b10001000, 0b10001100, 0b11001000, 0b11001100];

impl Ws2812SpiDevice {
    pub fn new(config: models::Ws2812Spi) -> Result<Self, DeviceError> {
        // Initialize SPI device
        let mut dev = Spidev::open(&config.output)?;
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(config.rate as _)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();
        dev.configure(&options)?;

        // Buffer for SPI tranfers
        let buf = vec![
            0;
            config.hardware_led_count as usize * SPI_BYTES_PER_LED
                + SPI_FRAME_END_LATCH_BYTES
        ];

        info!(path = %config.output, "initialized SPI device");

        Ok(Self {
            config,
            dev,
            buf,
            last_write_time: None,
        })
    }

    fn write(&mut self) -> Result<(), DeviceError> {
        // Perform SPI transfer
        let mut transfer = SpidevTransfer::write(&self.buf);
        self.dev.transfer(&mut transfer)?;

        // Update last write time
        self.last_write_time = Some(Instant::now());

        Ok(())
    }
}

#[async_trait]
impl DeviceImpl for Ws2812SpiDevice {
    async fn set_led_data(&mut self, led_data: &[models::Color]) -> Result<(), DeviceError> {
        // Update buffer
        let mut ptr = 0;
        for led in led_data {
            let (r, g, b) = self
                .config
                .color_order
                .reorder_from_rgb(*led)
                .into_components();
            let mut color_bits = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);

            for j in (0..SPI_BYTES_PER_LED).rev() {
                self.buf[ptr + j] = BITPAIR_TO_BYTE[(color_bits & 0x3) as usize];
                color_bits >>= 2;
            }

            ptr += SPI_BYTES_PER_LED;
        }

        for dst in self.buf.iter_mut().skip(ptr) {
            *dst = 0;
        }

        // Write immediately, update is only for refresh
        self.write()?;

        Ok(())
    }

    async fn update(&mut self) -> Result<(), super::DeviceError> {
        let now = Instant::now();
        let next_rewrite_time = self
            .last_write_time
            .map(|lwt| lwt + Duration::from_millis(self.config.rewrite_time as _))
            .unwrap_or(now);

        // Wait until the next rewrite cycle if necessary
        if next_rewrite_time > now {
            tokio::time::sleep_until(next_rewrite_time.into()).await;
        }

        // Write to device
        self.write()?;

        Ok(())
    }
}
