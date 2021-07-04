//! Boblight protocol server implementation

use std::net::SocketAddr;
use std::sync::Arc;

use futures::prelude::*;
use thiserror::Error;
use tokio::{net::TcpStream, sync::mpsc::Sender};
use tokio_util::codec::Framed;

use crate::{
    api::boblight::{self, BoblightApiError},
    global::{Global, InputMessage, InputSourceName},
    models::InstanceConfig,
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

pub async fn handle_client(
    (socket, peer_addr): (TcpStream, SocketAddr),
    tx: Sender<InputMessage>,
    instance: Arc<InstanceConfig>,
    global: Global,
) -> Result<(), BoblightServerError> {
    debug!("accepted new connection from {}", peer_addr,);

    let framed = Framed::new(socket, BoblightCodec::new());
    let (mut writer, mut reader) = framed.split();

    let source_handle = global
        .register_input_source(InputSourceName::Boblight { peer_addr }, None)
        .await
        .unwrap();
    let mut connection = boblight::ClientConnection::new(source_handle, tx, instance);

    while let Some(request) = reader.next().await {
        trace!("({}) processing request: {:?}", peer_addr, request);

        match request {
            Ok(request) => match connection.handle_request(request).await {
                Ok(response) => {
                    if let Some(response) = response {
                        writer.send(response).await?;
                    }
                }
                Err(error) => {
                    warn!("({}) boblight error: {}", peer_addr, error);
                }
            },
            Err(error) => {
                warn!("({}) boblight error: {}", peer_addr, error);
            }
        }
    }

    Ok(())
}
