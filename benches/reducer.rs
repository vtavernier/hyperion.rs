use std::convert::TryFrom;

use criterion::{criterion_group, criterion_main, Criterion};
use rand::prelude::*;

use hyperion::{
    image::*,
    models::{ClassicLedConfig, Color16, Leds, ToLeds},
};

fn random_image(width: u32, height: u32) -> RawImage {
    let mut data = vec![0u8; (width * height * RawImage::CHANNELS) as usize];

    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut data);

    RawImage::try_from((data, width, height)).unwrap()
}

fn classic_led_config(leds: u32) -> Leds {
    let classic_led_config = ClassicLedConfig {
        top: leds / 4,
        bottom: leds / 4,
        left: leds / 4,
        right: leds / 4,
        ..Default::default()
    };

    classic_led_config.to_leds()
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let width = 1920 / 16;
    let height = 1080 / 16;
    let leds = classic_led_config(40);
    let mut colors = vec![Color16::default(); leds.leds.len()];

    c.bench_function(
        &format!("{} px {} leds", width * height, leds.leds.len()),
        |b| {
            let mut reducer = Reducer::default();
            let image = random_image(width, height);

            b.iter(|| reducer.reduce(&image, &leds.leds, &mut colors))
        },
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
