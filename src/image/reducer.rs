use crate::models::{Color16, Led};

use super::Image;

#[derive(Debug, Default)]
pub struct Reducer {}

impl Reducer {
    pub fn reduce(&mut self, image: &impl Image, leds: &[Led], color_data: &mut [Color16]) {
        let width = image.width() as f32;
        let height = image.height() as f32;
        for (spec, value) in leds.iter().zip(color_data.iter_mut()) {
            let mut r_acc = 0u64;
            let mut g_acc = 0u64;
            let mut b_acc = 0u64;
            let mut cnt = 0u64;

            // TODO: Fixed point arithmetic
            let lxmin = spec.hmin * width;
            let lxmax = spec.hmax * width;
            let lymin = spec.vmin * height;
            let lymax = spec.vmax * height;

            for y in lymin.floor() as u32..=(lymax.ceil() as u32).min(image.height() - 1) {
                let y_area = if (y as f32) < lymin {
                    (255. * (1. - lymin.fract())) as u64
                } else if (y + 1) as f32 > lymax {
                    (255. * lymax.fract()) as u64
                } else {
                    255
                };

                for x in lxmin.floor() as u32..=(lxmax.ceil() as u32).min(image.width() - 1) {
                    // Safety: x (resp. y) are necessarily in 0..width (resp. 0..height)
                    let rgb = unsafe { image.color_at_unchecked(x, y) };
                    let x_area = if (x as f32) < lxmin {
                        (255. * (1. - lxmin.fract())) as u64
                    } else if (x + 1) as f32 > lxmax {
                        (255. * lxmax.fract()) as u64
                    } else {
                        255
                    };

                    let area = x_area * y_area / 255;

                    let (r, g, b) = rgb.into_components();
                    r_acc += (r as u64 * 255) * area;
                    g_acc += (g as u64 * 255) * area;
                    b_acc += (b as u64 * 255) * area;
                    cnt += area;
                }
            }

            *value = Color16::new(
                (r_acc / cnt.max(1)).max(0).min(u16::MAX as _) as u16,
                (g_acc / cnt.max(1)).max(0).min(u16::MAX as _) as u16,
                (b_acc / cnt.max(1)).max(0).min(u16::MAX as _) as u16,
            );
        }
    }
}
