use bytes::BytesMut;
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder, LinesCodec};

use crate::api::boblight::message;

#[derive(Debug, Error)]
pub enum BoblightCodecError {
    #[error("i/o error: {0}")]
    Io(#[from] futures_io::Error),
    #[error("lines codec error: {0}")]
    Lines(#[from] tokio_util::codec::LinesCodecError),
    #[error("invalid message: {0}")]
    Message(#[from] message::DecodeError),
}

/// JSON tokio codec
pub struct BoblightCodec {
    /// Line parsing codec
    lines: LinesCodec,
}

impl BoblightCodec {
    /// Create a new BoblightCodec
    pub fn new() -> Self {
        Self {
            lines: LinesCodec::new(),
        }
    }
}

impl Decoder for BoblightCodec {
    type Item = message::BoblightRequest;
    type Error = BoblightCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.lines.decode(src) {
            Ok(lines_result) => Ok(match lines_result {
                Some(ref line) => Some(line.parse()?),
                None => None,
            }),
            Err(error) => Err(error.into()),
        }
    }
}

impl Encoder<message::BoblightResponse> for BoblightCodec {
    type Error = BoblightCodecError;

    fn encode(
        &mut self,
        item: message::BoblightResponse,
        dst: &mut BytesMut,
    ) -> Result<(), Self::Error> {
        Ok(self.lines.encode(item.to_string(), dst)?)
    }
}
