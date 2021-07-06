use std::convert::TryFrom;

use thiserror::Error;

use crate::models::Color;

pub trait Image {
    /// Get the width of the image, in pixels
    fn width(&self) -> u32;

    /// Get the height of the image, in pixels
    fn height(&self) -> u32;

    /// Get the color at the given coordinates
    fn color_at(&self, x: u32, y: u32) -> Option<Color>;

    /// Get the color at the given coordinates skipping bound checks
    unsafe fn color_at_unchecked(&self, x: u32, y: u32) -> Color;

    /// Convert this image trait object to a raw image
    fn to_raw_image(&self) -> RawImage;
}

#[derive(Debug, Error)]
pub enum RawImageError {
    #[error("invalid data ({data} bytes) for the given dimensions ({width} x {height} x {channels} = {expected})")]
    InvalidData {
        data: usize,
        width: u32,
        height: u32,
        channels: u32,
        expected: usize,
    },
    #[error("invalid width")]
    InvalidWidth,
    #[error("invalid height")]
    InvalidHeight,
    #[error("raw image data missing")]
    RawImageMissing,
    #[error("image width is zero")]
    ZeroWidth,
    #[error("image height is zero")]
    ZeroHeight,
    #[error("i/o error")]
    Io(#[from] std::io::Error),
    #[error("encoding error")]
    Encoding(#[from] image::ImageError),
}

#[derive(Clone)]
pub struct RawImage {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

impl RawImage {
    const CHANNELS: u32 = 3;

    pub fn write_to_kitty(&self, out: &mut dyn std::io::Write) -> Result<(), RawImageError> {
        // Buffer for raw PNG data
        let mut buf = Vec::new();
        // PNG encoder
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        // Write PNG to buffer
        encoder.encode(
            &self.data[..],
            self.width,
            self.height,
            image::ColorType::Rgb8,
        )?;
        // Encode to base64
        let encoded = base64::encode(&buf);
        // Split into chunks
        let chunks = encoded.as_bytes().chunks(4096).collect::<Vec<_>>();
        // Transmit chunks
        for (i, chunk) in chunks.iter().enumerate() {
            let last = if i == chunks.len() - 1 { b"0" } else { b"1" };

            match i {
                0 => {
                    // First chunk
                    out.write_all(b"\x1B_Gf=100,a=T,m=")?;
                }
                _ => {
                    // Other chunks
                    out.write_all(b"\x1B_Gm=")?;
                }
            }

            out.write_all(last)?;
            out.write_all(b";")?;
            out.write_all(chunk)?;
            out.write_all(b"\x1B\\")?;
        }

        // Finish with new-line
        out.write_all(b"\n")?;

        Ok(())
    }
}

impl Image for RawImage {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn color_at(&self, x: u32, y: u32) -> Option<Color> {
        if x < self.width && y < self.height {
            unsafe {
                Some(Color::new(
                    *self
                        .data
                        .get_unchecked(((y * self.width + x) * Self::CHANNELS) as usize),
                    *self
                        .data
                        .get_unchecked(((y * self.width + x) * Self::CHANNELS + 1) as usize),
                    *self
                        .data
                        .get_unchecked(((y * self.width + x) * Self::CHANNELS + 2) as usize),
                ))
            }
        } else {
            None
        }
    }

    unsafe fn color_at_unchecked(&self, x: u32, y: u32) -> Color {
        Color::new(
            *self
                .data
                .get_unchecked(((y * self.width + x) * Self::CHANNELS) as usize),
            *self
                .data
                .get_unchecked(((y * self.width + x) * Self::CHANNELS + 1) as usize),
            *self
                .data
                .get_unchecked(((y * self.width + x) * Self::CHANNELS + 2) as usize),
        )
    }

    fn to_raw_image(&self) -> RawImage {
        self.clone()
    }
}

impl std::fmt::Debug for RawImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("RawImage");
        f.field("width", &self.width);
        f.field("height", &self.height);
        f.field("channels", &Self::CHANNELS);

        if self.data.len() > 32 {
            f.field("data", &format!("[{} bytes]", self.data.len()));
        } else {
            f.field("data", &self.data);
        }

        f.finish()
    }
}

impl TryFrom<(Vec<u8>, u32, u32)> for RawImage {
    type Error = RawImageError;

    fn try_from((data, width, height): (Vec<u8>, u32, u32)) -> Result<Self, Self::Error> {
        let expected = (width * height * Self::CHANNELS) as usize;

        if data.len() != expected {
            return Err(RawImageError::InvalidData {
                data: data.len(),
                width,
                height,
                channels: Self::CHANNELS,
                expected,
            });
        } else if width == 0 {
            return Err(RawImageError::ZeroWidth);
        } else if height == 0 {
            return Err(RawImageError::ZeroHeight);
        }

        Ok(Self {
            data,
            width,
            height,
        })
    }
}
