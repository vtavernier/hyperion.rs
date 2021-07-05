use bytes::BytesMut;
use prost::Message;
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

use crate::api::proto::message;

#[derive(Debug, Error)]
pub enum ProtoCodecError {
    #[error("i/o error: {0}")]
    Io(#[from] futures_io::Error),
    #[error(transparent)]
    LengthDelimited(#[from] tokio_util::codec::LengthDelimitedCodecError),
    #[error(transparent)]
    Decode(#[from] prost::DecodeError),
    #[error(transparent)]
    Encode(#[from] prost::EncodeError),
}

/// JSON tokio codec
pub struct ProtoCodec {
    /// Line parsing codec
    inner: LengthDelimitedCodec,
    /// Buffer for encoding messages
    buf: BytesMut,
}

impl ProtoCodec {
    /// Create a new ProtoCodec
    pub fn new() -> Self {
        Self {
            inner: LengthDelimitedCodec::builder()
                .length_field_length(4)
                .new_codec(),
            buf: BytesMut::new(),
        }
    }
}

impl Decoder for ProtoCodec {
    type Item = message::HyperionRequest;
    type Error = ProtoCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.inner.decode(src) {
            Ok(inner_result) => Ok(match inner_result {
                Some(ref data) => Some(message::HyperionRequest::decode(data.clone().freeze())?),
                None => None,
            }),
            Err(error) => Err(error.into()),
        }
    }
}

impl Encoder<message::HyperionReply> for ProtoCodec {
    type Error = ProtoCodecError;

    fn encode(
        &mut self,
        item: message::HyperionReply,
        dst: &mut BytesMut,
    ) -> Result<(), Self::Error> {
        self.buf.clear();
        self.buf.reserve(item.encoded_len());

        match item.encode(&mut self.buf) {
            Ok(_) => Ok(self.inner.encode(self.buf.clone().freeze(), dst)?),
            Err(encode_error) => Err(encode_error.into()),
        }
    }
}
