use bytes::BytesMut;
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder, LinesCodec};

use crate::api::json::message;

#[derive(Debug, Error)]
pub enum JsonCodecError {
    #[error("i/o error: {0}")]
    Io(#[from] futures_io::Error),
    #[error("lines codec error: {0}")]
    Lines(#[from] tokio_util::codec::LinesCodecError),
    #[error("invalid message: {0}")]
    Serde(#[from] serde_json::Error),
}

/// JSON tokio codec
pub struct JsonCodec {
    /// Line parsing codec
    lines: LinesCodec,
}

impl JsonCodec {
    /// Create a new JsonCodec
    pub fn new() -> Self {
        Self {
            lines: LinesCodec::new(),
        }
    }
}

impl Decoder for JsonCodec {
    type Item = message::HyperionMessage;
    type Error = JsonCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.lines.decode(src) {
            Ok(lines_result) => Ok(match lines_result {
                Some(ref line) => Some(serde_json::from_str(line)?),
                None => None,
            }),
            Err(error) => Err(error.into()),
        }
    }
}

impl Encoder<message::HyperionResponse> for JsonCodec {
    type Error = JsonCodecError;

    fn encode(
        &mut self,
        item: message::HyperionResponse,
        dst: &mut BytesMut,
    ) -> Result<(), Self::Error> {
        match serde_json::to_string(&item) {
            Ok(encoded) => Ok(self.lines.encode(encoded, dst)?),
            Err(encode_error) => Err(encode_error.into()),
        }
    }
}
