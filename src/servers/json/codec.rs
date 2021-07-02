use bytes::BytesMut;
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder, LinesCodec};

use crate::api::json::message;

/// Parse an incoming request as JSON into the corresponding message type
///
/// # Parameters
///
/// * `line`: input request line to parse as a message
///
/// # Errors
///
/// When the line cannot be parsed as JSON, the underlying error is returned from serde_json.
fn parse_request(line: &str) -> serde_json::Result<message::HyperionMessage> {
    serde_json::from_str(line)
}

/// Encode an outgoing reply as JSON
///
/// # Parameters
///
/// * `reply`: reply to encode as JSON
///
/// # Errors
///
/// When the reply cannot be encoded as JSON, the underlying error is returned from serde_json.
fn encode_reply(reply: &message::HyperionResponse) -> serde_json::Result<String> {
    serde_json::to_string(reply)
}

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
                Some(ref line) => Some(parse_request(line)?),
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
        match encode_reply(&item) {
            Ok(encoded) => Ok(self.lines.encode(encoded, dst)?),
            Err(encode_error) => Err(encode_error.into()),
        }
    }
}
