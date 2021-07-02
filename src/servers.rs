use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use tokio::net::{TcpListener, TcpStream};

use crate::{global::Global, models::ServerConfig};

pub mod flat;
pub mod json;
pub mod proto;

pub async fn bind<T, E, F>(
    options: T,
    global: Global,
    handle_client: impl Fn((TcpStream, SocketAddr), Global) -> F,
) -> Result<(), E>
where
    T: ServerConfig,
    F: futures::Future<Output = Result<(), E>> + Send + 'static,
    E: From<std::io::Error> + std::fmt::Display + Send + 'static,
{
    // Compute binding address
    let address = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), options.port());

    // Setup listener
    let listener = TcpListener::bind(&address).await?;

    // Notify we are listening
    info!("server listening on {}", address);

    loop {
        let incoming = listener.accept().await?;
        tokio::spawn({
            let peer_addr = incoming.1.clone();
            let ft = handle_client(incoming, global.clone());

            async move {
                let result = ft.await;

                match result {
                    Ok(_) => {
                        info!("({}) client disconnected", peer_addr);
                    }
                    Err(error) => {
                        error!("({}) client error:{}", peer_addr, error);
                    }
                }
            }
        });
    }
}
