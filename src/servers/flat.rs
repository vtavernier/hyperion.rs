//! flatbuffers flatcol server implementation

use std::net::SocketAddr;

use futures::prelude::*;
use thiserror::Error;
use tokio::net::TcpStream;

use crate::{
    api::flat::{self, message, FlatApiError},
    global::{Global, InputMessage, InputSourceHandle, PriorityGuard},
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
    priority_guard: &mut Option<PriorityGuard>,
) -> Result<(), FlatServerError> {
    let request = message::root_as_request(request_bytes.as_ref())?;

    trace!(request = ?request.command_type(), "processing");

    Ok(flat::handle_request(peer_addr, request, source, global, priority_guard).await?)
}

#[instrument(skip(socket, global))]
pub async fn handle_client(
    (socket, peer_addr): (TcpStream, SocketAddr),
    global: Global,
) -> Result<(), FlatServerError> {
    debug!("accepted new connection");

    let framed = tokio_util::codec::LengthDelimitedCodec::builder()
        .length_field_length(4)
        .new_framed(socket);

    let (mut writer, mut reader) = framed.split();

    let mut source = None;
    let mut priority_guard = None;
    let mut builder = flatbuffers::FlatBufferBuilder::new();

    while let Some(request_bytes) = reader.next().await {
        let request_bytes = match request_bytes {
            Ok(rb) => rb,
            Err(error) => {
                error!(error = %error, "error reading frame");
                continue;
            }
        };

        builder.reset();

        let reply = match handle_request(
            peer_addr,
            request_bytes,
            &mut source,
            &global,
            &mut priority_guard,
        )
        .await
        {
            Ok(()) => {
                if let Some(source) = source.as_ref() {
                    register_response(&mut builder, source.priority().unwrap())
                } else {
                    error_response(&mut builder, "unregistered source")
                }
            }
            Err(error) => {
                error!(error = %error, "error processing request");

                error_response(&mut builder, error)
            }
        };

        trace!(response = ?reply, "sending");
        writer.send(reply).await?;
        writer.flush().await?;
    }

    Ok(())
}
