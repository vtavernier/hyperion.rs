//! protobuf protocol server implementation

use std::net::SocketAddr;

use futures::prelude::*;
use prost::Message;
use thiserror::Error;
use tokio::net::TcpStream;

use crate::{
    api::proto::{self, message, ProtoApiError},
    global::{Global, InputMessage, InputSourceHandle, InputSourceName},
};

#[derive(Debug, Error)]
pub enum ProtoServerError {
    #[error("i/o error: {0}")]
    Io(#[from] futures_io::Error),
    #[error("decode error: {}", 0)]
    DecodeError(#[from] prost::DecodeError),
    #[error(transparent)]
    Api(#[from] ProtoApiError),
}

fn encode_response(buf: &mut bytes::BytesMut, msg: impl prost::Message) -> bytes::Bytes {
    // Clear the buffer to start fresh
    buf.clear();

    // Reserve enough space for the response
    let len = msg.encoded_len();
    if buf.capacity() < len {
        buf.reserve(len * 2);
    }

    // Encode the message
    msg.encode(buf).unwrap();
    buf.split().freeze()
}

fn success_response(peer_addr: SocketAddr, buf: &mut bytes::BytesMut) -> bytes::Bytes {
    let mut reply = message::HyperionReply::default();
    reply.r#type = message::hyperion_reply::Type::Reply.into();
    reply.success = Some(true);

    trace!("({}) sending success: {:?}", peer_addr, reply);
    encode_response(buf, reply)
}

fn error_response(
    peer_addr: SocketAddr,
    buf: &mut bytes::BytesMut,
    error: impl std::fmt::Display,
) -> bytes::Bytes {
    let mut reply = message::HyperionReply::default();
    reply.r#type = message::hyperion_reply::Type::Reply.into();
    reply.success = Some(false);
    reply.error = Some(error.to_string());

    trace!("({}) sending error: {:?}", peer_addr, reply);
    encode_response(buf, reply)
}

fn handle_request(
    peer_addr: SocketAddr,
    request_bytes: bytes::BytesMut,
    source: &InputSourceHandle<InputMessage>,
) -> Result<(), ProtoServerError> {
    let request_bytes = request_bytes.freeze();
    let request = message::HyperionRequest::decode(request_bytes.clone())?;

    trace!("({}) got request: {:?}", peer_addr, request);

    Ok(proto::handle_request(request, source)?)
}

pub async fn handle_client(
    (socket, peer_addr): (TcpStream, SocketAddr),
    global: Global,
) -> Result<(), ProtoServerError> {
    debug!("accepted new connection from {}", peer_addr);

    let framed = tokio_util::codec::LengthDelimitedCodec::builder()
        .length_field_length(4)
        .new_framed(socket);
    let (mut writer, mut reader) = framed.split();

    // unwrap: cannot fail because the priority is None
    let source = global
        .register_input_source(InputSourceName::Protobuf { peer_addr }, None)
        .await
        .unwrap();

    // buffer for building responses
    let mut reply_buf = bytes::BytesMut::with_capacity(128);

    while let Some(request_bytes) = reader.next().await {
        let request_bytes = match request_bytes {
            Ok(rb) => rb,
            Err(error) => {
                error!("({}) error reading frame: {}", peer_addr, error);
                continue;
            }
        };

        let reply = match handle_request(peer_addr, request_bytes, &source) {
            Ok(()) => success_response(peer_addr, &mut reply_buf),
            Err(error) => {
                error!("({}) error processing request: {}", peer_addr, error);

                error_response(peer_addr, &mut reply_buf, error)
            }
        };

        writer.send(reply).await?;
    }

    Ok(())
}
