//! flatbuffers flatcol server implementation

use std::net::SocketAddr;

use futures::prelude::*;
use thiserror::Error;
use tokio::net::TcpStream;

use crate::{
    api::flat::{self, message, FlatApiError},
    global::{Global, InputMessage, InputSourceHandle},
};

#[derive(Debug, Error)]
pub enum FlatServerError {
    #[error("i/o error: {0}")]
    Io(#[from] futures_io::Error),
    #[error("error decoding frame")]
    FlatBuffer(#[from] flatbuffers::InvalidFlatbuffer),
    #[error(transparent)]
    Api(#[from] FlatApiError),
}

fn register_response(builder: &mut flatbuffers::FlatBufferBuilder, priority: i32) -> bytes::Bytes {
    let mut reply = message::ReplyBuilder::new(builder);
    reply.add_registered(priority);

    let reply = reply.finish();

    builder.finish(reply, None);
    bytes::Bytes::copy_from_slice(builder.finished_data())
}

fn error_response(
    builder: &mut flatbuffers::FlatBufferBuilder,
    error: impl std::fmt::Display,
) -> bytes::Bytes {
    let error = builder.create_string(error.to_string().as_str());

    let mut reply = message::ReplyBuilder::new(builder);
    reply.add_error(error);

    let reply = reply.finish();

    builder.finish(reply, None);
    bytes::Bytes::copy_from_slice(builder.finished_data())
}

async fn handle_request(
    peer_addr: SocketAddr,
    request_bytes: bytes::BytesMut,
    source: &mut Option<InputSourceHandle<InputMessage>>,
    global: &Global,
) -> Result<(), FlatServerError> {
    let request = message::root_as_request(request_bytes.as_ref())?;

    trace!("({}) got request: {:?}", peer_addr, request.command_type());

    Ok(flat::handle_request(peer_addr, request, source, global).await?)
}

pub async fn handle_client(
    (socket, peer_addr): (TcpStream, SocketAddr),
    global: Global,
) -> Result<(), FlatServerError> {
    debug!("accepted new connection from {}", peer_addr);

    let framed = tokio_util::codec::LengthDelimitedCodec::builder()
        .length_field_length(4)
        .new_framed(socket);

    let (mut writer, mut reader) = framed.split();

    let mut source = None;
    let mut builder = flatbuffers::FlatBufferBuilder::new();

    while let Some(request_bytes) = reader.next().await {
        let request_bytes = match request_bytes {
            Ok(rb) => rb,
            Err(error) => {
                error!("({}) error reading frame: {}", peer_addr, error);
                continue;
            }
        };

        builder.reset();

        let reply = match handle_request(peer_addr, request_bytes, &mut source, &global).await {
            Ok(()) => {
                if let Some(source) = source.as_ref() {
                    register_response(&mut builder, source.priority().unwrap())
                } else {
                    error_response(&mut builder, "unregistered source")
                }
            }
            Err(error) => {
                error!("({}) error processing request: {}", peer_addr, error);

                error_response(&mut builder, error)
            }
        };

        trace!("sending response: {:?}", reply);
        writer.send(reply).await?;
    }

    Ok(())
}
