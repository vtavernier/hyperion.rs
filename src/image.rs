use std::convert::TryFrom;

use thiserror::Error;

use crate::models::Color;

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
    channels: u32,
}

impl RawImage {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn channels(&self) -> u32 {
        self.channels
    }

    pub fn color_at(&self, x: u32, y: u32) -> Option<Color> {
        if x < self.width && y < self.height && self.channels >= 3 {
            unsafe {
                Some(Color::from_components((
                    *self
                        .data
                        .get_unchecked(((y * self.width + x) * self.channels) as usize),
                    *self
                        .data
                        .get_unchecked(((y * self.width + x) * self.channels + 1) as usize),
                    *self
                        .data
                        .get_unchecked(((y * self.width + x) * self.channels + 2) as usize),
                )))
            }
        } else {
            None
        }
    }

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
            if self.channels == 3 {
                image::ColorType::Rgb8
            } else if self.channels == 4 {
                image::ColorType::Rgba8
            } else {
                panic!();
            },
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

impl std::fmt::Debug for RawImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("RawImage");
        f.field("width", &self.width);
        f.field("height", &self.height);
        f.field("channels", &self.channels);

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
        let channels = 3;
        let expected = (width * height * channels) as usize;

        if data.len() != expected {
            return Err(RawImageError::InvalidData {
                data: data.len(),
                width,
                height,
                channels,
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
            channels,
        })
    }
}
