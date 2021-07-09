//! protobuf protocol server implementation

use std::net::SocketAddr;

use futures::prelude::*;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use crate::{
    api::proto::{self, message, ProtoApiError},
    global::{Global, InputSourceName, PriorityGuard},
};

mod codec;
use codec::*;

#[derive(Debug, Error)]
pub enum ProtoServerError {
    #[error("i/o error: {0}")]
    Io(#[from] futures_io::Error),
    #[error("decode error: {0}")]
    Codec(#[from] ProtoCodecError),
    #[error(transparent)]
    Api(#[from] ProtoApiError),
}

fn success_response(peer_addr: SocketAddr) -> message::HyperionReply {
    let mut reply = message::HyperionReply::default();
    reply.r#type = message::hyperion_reply::Type::Reply.into();
    reply.success = Some(true);

    trace!("({}) sending success: {:?}", peer_addr, reply);
    reply
}

fn error_response(peer_addr: SocketAddr, error: impl std::fmt::Display) -> message::HyperionReply {
    let mut reply = message::HyperionReply::default();
    reply.r#type = message::hyperion_reply::Type::Reply.into();
    reply.success = Some(false);
    reply.error = Some(error.to_string());

    trace!("({}) sending error: {:?}", peer_addr, reply);
    reply
}

pub async fn handle_client(
    (socket, peer_addr): (TcpStream, SocketAddr),
    global: Global,
) -> Result<(), ProtoServerError> {
    debug!("accepted new connection from {}", peer_addr);

    let (mut writer, mut reader) = Framed::new(socket, ProtoCodec::new()).split();

    // unwrap: cannot fail because the priority is None
    let source = global
        .register_input_source(InputSourceName::Protobuf { peer_addr }, None)
        .await
        .unwrap();

    let mut priority_guard = PriorityGuard::new_broadcast(&source);

    while let Some(request) = reader.next().await {
        let request = match request {
            Ok(rb) => rb,
            Err(error) => {
                error!("({}) error reading frame: {}", peer_addr, error);
                continue;
            }
        };

        trace!("({}) got request: {:?}", peer_addr, request);

        let reply =
            match proto::handle_request(peer_addr.clone(), request, &source, &mut priority_guard) {
                Ok(()) => success_response(peer_addr),
                Err(error) => {
                    error!("({}) error processing request: {}", peer_addr, error);

                    error_response(peer_addr, error)
                }
            };

        writer.send(reply).await?;
        writer.flush().await?;
    }

    Ok(())
}
