//! Definition of the Processor type

// TODO: Implement black border detection
// TODO: Use SIMD

use crate::color;
use std::cmp::min;

use super::RawImage;

/// Image pixel accumulator
#[derive(Default, Clone)]
struct Pixel {
    /// Accumulated color
    color: [f32; 3],
    /// Number of samples
    count: f32,
}

impl Pixel {
    /// Reset this pixels' value and sample count
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Add a new sample to this pixel
    ///
    /// # Parameters
    ///
    /// * `(r, g, b)`: sampled RGB values
    /// * `area_factor`: weight of the current sample. 1.0 is the weight of a sample which covers
    /// the entire matching LED area.
    pub fn sample(&mut self, (r, g, b): (u8, u8, u8), mut area_factor: f32) {
        let min = 0.0;
        let max = 1.0;

        if area_factor < min {
            area_factor = min;
        } else if area_factor > max {
            area_factor = max;
        }

        self.color[0] += area_factor * f32::from(r) / 255.0f32;
        self.color[1] += area_factor * f32::from(g) / 255.0f32;
        self.color[2] += area_factor * f32::from(b) / 255.0f32;
        self.count += area_factor;
    }

    /// Compute the mean of this pixel
    pub fn mean(&self) -> color::ColorPoint {
        let rgb: (f32, f32, f32) = (
            self.color[0] / self.count,
            self.color[1] / self.count,
            self.color[2] / self.count,
        );

        color::ColorPoint::from(rgb)
    }
}

/// Raw image data processor
#[derive(Default)]
pub struct Processor {
    /// Width of the LED map
    width: usize,
    /// Height of the LED map
    height: usize,
    // TODO: use a proper 2D interval tree
    /// 2D row-major list of (color_idx, device_idx, led_idx, area_factor)
    led_map: Vec<Vec<(usize, usize, usize, f32)>>,
    /// Color storage for every known LED
    color_map: Vec<Pixel>,
}

/// Image processor reference with LED details
pub struct ProcessorWithDevices<'p, 'a, I: Iterator<Item = &'a crate::config::Device>> {
    /// Image processor
    processor: &'p mut Processor,
    /// Devices iterator
    devices: I,
}

impl Processor {
    /// Allocates the image processor working memory
    ///
    /// # Parameters
    ///
    /// * `width`: width of the incoming images in pixels
    /// * `height`: height of the incoming images in pixels
    /// * `leds`: LED specification for target devices
    fn alloc<'a>(
        &mut self,
        width: usize,
        height: usize,
        devices: impl Iterator<Item = &'a crate::config::Device>,
    ) {
        // Initialize led map data structure
        let mut led_map = Vec::with_capacity(width * height);
        for _ in 0..(width * height) {
            led_map.push(Vec::new());
        }

        // Add leds whose area overlap the current pixel
        let mut index = 0;
        for (device_idx, device) in devices.enumerate() {
            for (led_idx, led) in device.leds.iter().enumerate() {
                for j in min(height - 1, (led.vscan.min * height as f32).floor() as usize)
                    ..min(height, (led.vscan.max * height as f32).ceil() as usize)
                {
                    // Vertical scan range
                    let y_min = j as f32 / height as f32;
                    let y_max = (j + 1) as f32 / height as f32;

                    for i in min(width - 1, (led.hscan.min * width as f32).floor() as usize)
                        ..min(width, (led.hscan.max * width as f32).ceil() as usize)
                    {
                        // Horizontal scan range
                        let x_min = i as f32 / width as f32;
                        let x_max = (i + 1) as f32 / width as f32;

                        let map_index = j * width + i;

                        if led.hscan.min < x_max
                            && led.hscan.max >= x_min
                            && led.vscan.min < y_max
                            && led.vscan.max >= y_min
                        {
                            // Compute area of pixel covered by scan parameter
                            let x_range = x_max.min(led.hscan.max) - x_min.max(led.hscan.min);
                            let y_range = y_max.min(led.vscan.max) - y_min.max(led.vscan.min);
                            let area = x_range * y_range;
                            let factor = area * (width * height) as f32;

                            led_map[map_index].push((index, device_idx, led_idx, factor));
                        }
                    }
                }

                index += 1;
            }
        }

        // Color map for computation
        let color_map = vec![Pixel::default(); index];

        *self = Self {
            width,
            height,
            led_map,
            color_map,
        };
    }

    /// Checks if this image processor is set up to process images of the given size
    ///
    /// # Parameters
    ///
    /// * `width`: width of the image data
    /// * `height`: height of the image data
    ///
    /// # Return value
    ///
    /// `true` if this image process supports this size, `false` otherwise
    fn matches(&self, width: usize, height: usize) -> bool {
        self.width == width && self.height == height
    }

    /// Prepares processing of image data using the given LED scan ranges
    ///
    /// Note that if changes occurred in the LED details, the image processor
    /// should be reset first so this method can reallocate internal structures.
    ///
    /// # Parameters
    ///
    /// * `devices`: iterator returning device specifications
    pub fn with_devices<'p, 'a, I: Iterator<Item = &'a crate::config::Device>>(
        &'p mut self,
        devices: I,
    ) -> ProcessorWithDevices<'p, 'a, I> {
        ProcessorWithDevices {
            processor: self,
            devices,
        }
    }

    /// Process incoming image data into led colors
    ///
    /// The image processor should already allocated at the right size
    /// using the alloc method.
    ///
    /// # Parameters
    ///
    /// * `raw_image`: raw RGB image
    fn process_image(&mut self, raw_image: RawImage) {
        let (width, height) = raw_image.get_dimensions();

        // Reset all colors
        for color in &mut self.color_map {
            color.reset();
        }

        for j in 0..height {
            for i in 0..width {
                let map_idx = (j * width + i) as usize;
                let rgb = raw_image.get_pixel(i, j);

                for (pixel_idx, _device_idx, _led_idx, area_factor) in &self.led_map[map_idx] {
                    self.color_map[*pixel_idx].sample(rgb, *area_factor);
                }
            }
        }
    }

    /// Update LEDs with computed colors
    ///
    /// # Parameters
    ///
    /// * `led_setter`: callback to update LED colors
    pub fn update_leds(&self, mut led_setter: impl FnMut((usize, usize), color::ColorPoint)) {
        // Compute mean and assign to led instances
        for pixel in self.led_map.iter() {
            for (pixel_idx, device_idx, led_idx, _area_factor) in pixel.iter() {
                led_setter((*device_idx, *led_idx), self.color_map[*pixel_idx].mean());
            }
        }
    }
}

impl<'p, 'a, I: Iterator<Item = &'a crate::config::Device>> ProcessorWithDevices<'p, 'a, I> {
    /// Process incoming image data into led colors
    ///
    /// # Parameters
    ///
    /// * `raw_image`: raw RGB image
    pub fn process_image(self, raw_image: RawImage) -> &'p mut Processor {
        let (width, height) = raw_image.get_dimensions();

        // Check that this processor has the right size
        if !self.processor.matches(width, height) {
            self.processor.alloc(width, height, self.devices);
        }

        self.processor.process_image(raw_image);
        self.processor
    }
}