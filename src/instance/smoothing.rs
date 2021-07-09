use std::time::{Duration, Instant};

use crate::models;

// TODO: Implement decay smoothing
// TODO: Implement dithering

pub struct Smoothing {
    config: models::Smoothing,
    led_data: Vec<models::Color>,
    current_data: Vec<models::Color16>,
    target_data: Vec<models::Color16>,
    target_time: Instant,
    previous_write_time: Instant,
    next_update: Option<Instant>,
}

impl Smoothing {
    pub fn new(config: models::Smoothing, led_count: usize) -> Self {
        let now = Instant::now();

        Self {
            config,
            led_data: vec![Default::default(); led_count],
            current_data: vec![Default::default(); led_count],
            target_data: vec![Default::default(); led_count],
            target_time: now,
            previous_write_time: now,
            next_update: None,
        }
    }

    /// Given the current time, prepare the next update
    fn plan_update(&mut self, now: Instant) -> SmoothingUpdate {
        if self.config.enable && now < self.target_time {
            // Smoothing enabled, the continuous update should happen at that time
            let next_update = self.next_update.unwrap_or(
                now + Duration::from_micros(
                    1_000_000_000 / (1000. * self.config.update_frequency) as u64,
                ),
            );

            self.next_update = Some(next_update);

            // Compute the led data for the current time
            let delta_time = (self.target_time - now).as_micros() as u64;
            let k = 1.
                - 1. * (delta_time as f32)
                    / (self.target_time - self.previous_write_time).as_micros() as f32;

            // Update current data with linear smoothing
            for (tgt, prev) in self.target_data.iter().zip(self.current_data.iter_mut()) {
                let r_diff = tgt.red as i32 - prev.red as i32;
                let g_diff = tgt.green as i32 - prev.green as i32;
                let b_diff = tgt.blue as i32 - prev.blue as i32;

                prev.red = (prev.red as i32 + r_diff.signum() * (k * r_diff.abs() as f32) as i32)
                    .max(0)
                    .min(65535) as u16;
                prev.green = (prev.green as i32
                    + g_diff.signum() * (k * g_diff.abs() as f32) as i32)
                    .max(0)
                    .min(65535) as u16;
                prev.blue = (prev.blue as i32 + b_diff.signum() * (k * b_diff.abs() as f32) as i32)
                    .max(0)
                    .min(65535) as u16;
            }
        } else {
            // Smoothing disabled, update as soon as possible
            if self.config.enable {
                self.next_update = None;
            } else {
                // Or linear update complete, color is stable
                self.next_update = Some(now);
            }

            // Update current data from target data
            self.current_data.copy_from_slice(&self.target_data);
        }

        // Convert current data to led data
        for (src, dst) in self.current_data.iter().zip(self.led_data.iter_mut()) {
            *dst = crate::color::color_to8(*src);
        }

        if self.next_update.is_some() {
            SmoothingUpdate::Running
        } else {
            SmoothingUpdate::Settled
        }
    }

    pub fn set_target(&mut self, color_data: &[models::Color16]) {
        // Update our copy of the target data
        self.target_data.copy_from_slice(color_data);

        // Update times
        let now = Instant::now();
        self.previous_write_time = now;
        self.target_time = now + Duration::from_millis(self.config.time_ms as _);

        self.plan_update(now);
    }

    pub async fn update(&mut self) -> (&[models::Color], SmoothingUpdate) {
        if let Some(next_update) = self.next_update {
            // Wait for the right update time
            if next_update > Instant::now() {
                tokio::time::sleep_until(next_update.into()).await
            }

            // We waited until the update time, return the result and plan the next update
            self.next_update = None;
            let update = self.plan_update(Instant::now());

            (&self.led_data, update)
        } else {
            // No update pending
            futures::future::pending().await
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmoothingUpdate {
    Running,
    Settled,
}
