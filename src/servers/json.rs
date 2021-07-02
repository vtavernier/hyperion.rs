//! JSON protocol server implementation

use std::net::SocketAddr;

use futures::prelude::*;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use crate::{
    api::json::{self, JsonApiError},
    global::Global,
};

/// JSON protocol codec definition
mod codec;
use codec::*;

#[derive(Debug, Error)]
pub enum JsonServerError {
    #[error("i/o error: {0}")]
    Io(#[from] futures_io::Error),
    #[error("codec error: {0}")]
    Codec(#[from] JsonCodecError),
    #[error(transparent)]
    Api(#[from] JsonApiError),
}

pub async fn handle_client(
    (socket, peer_addr): (TcpStream, SocketAddr),
    global: Global,
) -> Result<(), JsonServerError> {
    debug!("accepted new connection from {}", peer_addr,);

    let framed = Framed::new(socket, JsonCodec::new());
    let (mut writer, mut reader) = framed.split();

    // unwrap: cannot fail because the priority is None
    let source = global
        .register_input_source(format!("JSON({})", peer_addr), None)
        .await
        .unwrap();

    while let Some(request) = reader.next().await {
        trace!("({}) processing request: {:?}", peer_addr, request);

        let mut tan = None;
        let reply = match {
            match request {
                Ok(rq) => {
                    tan = rq.tan;
                    Ok(json::handle_request(rq, &source, &global).await?)
                }
                Err(error) => Err(JsonServerError::from(error)),
            }
        } {
            Ok(None) => json::message::HyperionResponse::success(tan),
            Ok(Some(response)) => response,
            Err(error) => {
                error!("({}) error processing request: {}", peer_addr, error);

                json::message::HyperionResponse::error(tan, &error)
            }
        };

        trace!("({}) sending response: {:?}", peer_addr, reply);

        writer.send(reply).await?;
    }

    Ok(())
}
