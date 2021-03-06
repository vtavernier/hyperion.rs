use std::net::SocketAddr;

use futures::{Future, SinkExt, StreamExt};
use warp::{http::StatusCode, path::FullPath, Filter, Rejection};

use crate::{
    api::json::message,
    global::{Global, Paths},
    models::WebConfig,
};

mod session;
use session::*;

pub async fn bind(
    global: Global,
    config: &WebConfig,
    paths: &Paths,
) -> Result<impl Future<Output = ()>, std::io::Error> {
    let session_store = SessionStore::new(config.max_sessions as _);

    let ws = warp::ws()
        .and(session_store.request())
        .and(warp::filters::addr::remote())
        .and({
            let global = global.clone();
            warp::any().map(move || global.clone())
        })
        .map(
            |ws: warp::ws::Ws,
             session: SessionInstance,
             _remote: Option<SocketAddr>,
             global: Global| {
                (
                    ws.on_upgrade({
                        let session = session.session().clone();

                        move |websocket| {
                            // Just echo all messages back...
                            let (mut tx, mut rx) = websocket.split();

                            async move {
                                while let Some(result) = rx.next().await {
                                    if let Some(message) =
                                        session.write().await.handle_result(&global, result).await
                                    {
                                        if let Err(error) = tx.send(message).await {
                                            warn!(error = %error, "websocket error");
                                        }
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                    }),
                    session,
                )
            },
        )
        .untuple_one()
        .and_then(reply_session);

    let cgi = warp::path("cgi").and(
        warp::path("cfg_jsonserver")
            .and_then({
                let global = global.clone();
                move || {
                    let global = global.clone();

                    async move {
                        Ok::<_, Rejection>(format!(
                            ":{}",
                            global
                                .read_config(|config| config.global.json_server.port)
                                .await
                        ))
                    }
                }
            })
            .or(warp::path("run")
                .and(warp::path::full())
                .map(|full_path: FullPath| {
                    // TODO: Implement run?
                    warp::reply::with_status(
                        format!("script failed ({})", full_path.as_str()),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })),
    );

    let json_rpc = warp::path("json-rpc")
        .and(warp::body::json())
        .and(warp::filters::header::optional("Authorization"))
        .and(session_store.request())
        .and(warp::filters::addr::remote())
        .and(warp::any().map(move || global.clone()))
        .and_then(
            |request: message::HyperionMessage,
             _authorization: Option<String>,
             session: SessionInstance,
             _remote: Option<SocketAddr>,
             global: Global| {
                async move {
                    let reply = warp::reply::json(
                        &session
                            .session()
                            .write()
                            .await
                            .handle_request(&global, request)
                            .await,
                    );

                    Ok::<_, Rejection>((reply, session))
                }
            },
        )
        .untuple_one()
        .and_then(reply_session);

    let files = warp::fs::dir(paths.resolve_path(if config.document_root.is_empty() {
        WebConfig::SYSTEM_DOCUMENT_ROOT
    } else {
        config.document_root.as_str()
    }));

    // TODO: Serve error pages from /errorpages/*

    let address = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(address).await;

    match listener {
        Ok(listener) => {
            info!(address = %address, "Webconfig server listening");
            Ok(warp::serve(
                ws.or(cgi)
                    .or(json_rpc)
                    .or(files)
                    .with(warp::filters::log::log("hyperion::web")),
            )
            .run_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener)))
        }
        Err(error) => Err(error),
    }
}
