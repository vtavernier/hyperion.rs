//! Boblight protocol server implementation

use std::net::SocketAddr;

use futures::prelude::*;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use crate::{
    api::boblight::{self, BoblightApiError},
    global::{Global, InputSourceName},
    instance::InstanceHandle,
};

/// Boblight protocol codec definition
mod codec;
use codec::*;

#[derive(Debug, Error)]
pub enum BoblightServerError {
    #[error("i/o error: {0}")]
    Io(#[from] futures_io::Error),
    #[error("codec error: {0}")]
    Codec(#[from] BoblightCodecError),
    #[error(transparent)]
    Api(#[from] BoblightApiError),
}

#[instrument(skip(socket, led_count, instance, global))]
pub async fn handle_client(
    (socket, peer_addr): (TcpStream, SocketAddr),
    led_count: usize,
    instance: InstanceHandle,
    global: Global,
) -> Result<(), BoblightServerError> {
    debug!("accepted new connection");

    let framed = Framed::new(socket, BoblightCodec::new());
    let (mut writer, mut reader) = framed.split();

    let source_handle = global
        .register_input_source(InputSourceName::Boblight { peer_addr }, None)
        .await
        .unwrap();

    let mut connection = boblight::ClientConnection::new(source_handle, led_count, instance);

    while let Some(request) = reader.next().await {
        trace!(request = ?request, "processing");

        match request {
            Ok(request) => match connection.handle_request(request).await {
                Ok(response) => {
                    if let Some(response) = response {
                        writer.send(response).await?;
                    }
                }
                Err(error) => {
                    warn!(error = %error, "boblight error");
                }
            },
            Err(error) => {
                warn!(error = %error, "boblight error");
            }
        }
    }

    Ok(())
}
