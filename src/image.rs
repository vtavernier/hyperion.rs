use std::convert::TryFrom;

use thiserror::Error;

use crate::models::Color;

mod reducer;
pub use reducer::*;

pub trait Image: Sized {
    /// Get the width of the image, in pixels
    fn width(&self) -> u16;

    /// Get the height of the image, in pixels
    fn height(&self) -> u16;

    /// Get the color at the given coordinates
    fn color_at(&self, x: u16, y: u16) -> Option<Color>;

    /// Get the color at the given coordinates skipping bound checks
    unsafe fn color_at_unchecked(&self, x: u16, y: u16) -> Color;

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
    width: u16,
    height: u16,
}

impl RawImage {
    pub const CHANNELS: u16 = 3;

    pub fn write_to_kitty(&self, out: &mut dyn std::io::Write) -> Result<(), RawImageError> {
        // Buffer for raw PNG data
        let mut buf = Vec::new();
        // PNG encoder
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        // Write PNG to buffer
        encoder.encode(
            &self.data[..],
            self.width as _,
            self.height as _,
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
    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn color_at(&self, x: u16, y: u16) -> Option<Color> {
        if x < self.width && y < self.height {
            unsafe { Some(self.color_at_unchecked(x, y)) }
        } else {
            None
        }
    }

    unsafe fn color_at_unchecked(&self, x: u16, y: u16) -> Color {
        let idx = (y as usize * self.width as usize + x as usize) * Self::CHANNELS as usize;
        Color::new(
            *self.data.get_unchecked(idx),
            *self.data.get_unchecked(idx + 1),
            *self.data.get_unchecked(idx + 2),
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
        let expected = width as usize * height as usize * Self::CHANNELS as usize;

        if data.len() != expected {
            return Err(RawImageError::InvalidData {
                data: data.len(),
                width,
                height,
                channels: Self::CHANNELS as _,
                expected,
            });
        } else if width == 0 {
            return Err(RawImageError::ZeroWidth);
        } else if height == 0 {
            return Err(RawImageError::ZeroHeight);
        } else if width >= u16::MAX as _ {
            return Err(RawImageError::InvalidWidth);
        } else if height >= u16::MAX as _ {
            return Err(RawImageError::InvalidHeight);
        }

        Ok(Self {
            data,
            width: width as _,
            height: height as _,
        })
    }
}

pub struct ImageView<'i, T: Image> {
    inner: &'i T,
    xmin: u16,
    xmax: u16,
    ymin: u16,
    ymax: u16,
}

impl<'i, T: Image> Image for ImageView<'i, T> {
    fn width(&self) -> u16 {
        self.xmax - self.xmin
    }

    fn height(&self) -> u16 {
        self.ymax - self.ymin
    }

    fn color_at(&self, x: u16, y: u16) -> Option<Color> {
        self.inner.color_at(x + self.xmin, y + self.ymin)
    }

    unsafe fn color_at_unchecked(&self, x: u16, y: u16) -> Color {
        self.inner.color_at_unchecked(x + self.xmin, y + self.ymin)
    }

    fn to_raw_image(&self) -> RawImage {
        let w = self.width();
        let h = self.height();
        let mut data = Vec::with_capacity(w as usize * h as usize * RawImage::CHANNELS as usize);

        unsafe {
            for y in 0..h {
                for x in 0..w {
                    let (r, g, b) = self.color_at_unchecked(x, y).into_components();
                    data.push(r);
                    data.push(g);
                    data.push(b);
                }
            }
        }

        RawImage {
            data,
            width: w,
            height: h,
        }
    }
}

pub trait ImageViewExt: Image {
    fn wrap(&self, x: std::ops::Range<u16>, y: std::ops::Range<u16>) -> ImageView<Self>;
}

impl<T: Image> ImageViewExt for T {
    fn wrap(&self, x: std::ops::Range<u16>, y: std::ops::Range<u16>) -> ImageView<Self> {
        ImageView {
            inner: self,
            xmin: x.start,
            xmax: x.end,
            ymin: y.start,
            ymax: y.end,
        }
    }
}

pub mod prelude {
    pub use super::{Image, ImageViewExt};
}
