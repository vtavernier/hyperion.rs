use std::{collections::HashMap, convert::TryInto, fmt::Display, sync::Arc};

use thiserror::Error;
use tokio::sync::RwLock;
use warp::{ws::Message, Filter, Rejection, Reply};

use crate::{
    api::json::{
        message::{HyperionMessage, HyperionResponse},
        ClientConnection, JsonApiError,
    },
    global::{Global, InputSourceError},
};

#[derive(Debug, Error)]
pub enum SessionError {
    #[error(transparent)]
    InputSource(#[from] InputSourceError),
    #[error(transparent)]
    Api(#[from] JsonApiError),
    #[error("not implemented")]
    NotImplemented,
    #[error("invalid request: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Default, Debug)]
pub struct Session {
    id: uuid::Uuid,
    json_api: Option<ClientConnection>,
}

impl Session {
    async fn json_api(&mut self, global: &Global) -> Result<&mut ClientConnection, SessionError> {
        if self.json_api.is_none() {
            // Generate a session ID
            if self.id.is_nil() {
                self.id = uuid::Uuid::new_v4();
            }

            // Can't use SocketAddr, see https://github.com/seanmonstar/warp/issues/830
            self.json_api = Some(ClientConnection::new(
                global
                    .register_input_source(
                        crate::global::InputSourceName::Web {
                            session_id: self.id,
                        },
                        None,
                    )
                    .await?,
            ));
        }

        Ok(self.json_api.as_mut().unwrap())
    }

    async fn handle_message(
        &mut self,
        global: &Global,
        message: Message,
    ) -> Result<Message, SessionError> {
        let json_api = self.json_api(global).await?;

        if message.is_text() {
            let request: HyperionMessage = serde_json::from_str(message.to_str().unwrap())?;
            let response = json_api.handle_request(request, global).await?;
            return Ok(Message::text(serde_json::to_string(&response).unwrap()));
        }

        Err(SessionError::NotImplemented)
    }

    fn error_message<T: Display>(&self, e: T) -> Message {
        Message::text(
            serde_json::to_string(&serde_json::json!({ "error": e.to_string() })).unwrap(),
        )
    }

    #[instrument(skip(global, result))]
    pub async fn handle_result(
        &mut self,
        global: &Global,
        result: Result<Message, warp::Error>,
    ) -> Option<Message> {
        match result {
            Ok(message) => {
                trace!(message = ?message, "ws message");

                if message.is_close() {
                    return None;
                }

                let response = self.handle_message(&global, message).await;

                trace!(response = ?response, "ws response");

                match response {
                    Ok(message) => Some(message),
                    Err(error) => Some(self.error_message(error)),
                }
            }
            Err(error) => Some(self.error_message(error)),
        }
    }

    #[instrument(skip(global, request))]
    pub async fn handle_request(
        &mut self,
        global: &Global,
        request: HyperionMessage,
    ) -> HyperionResponse {
        trace!(request = ?request, "JSON RPC request");

        let tan = request.tan;
        let api = match self.json_api(global).await {
            Ok(api) => api,
            Err(error) => {
                return HyperionResponse::error(tan, &error);
            }
        };

        let response = match api.handle_request(request, global).await {
            Ok(None) => HyperionResponse::success(tan),
            Ok(Some(response)) => response,
            Err(error) => {
                error!(error =  %error, "error processing request");
                HyperionResponse::error(tan, &error)
            }
        };

        trace!(response = ?response, "ws response");
        response
    }
}

const COOKIE_NAME: &str = "hyperion_rs_sid";

type SessionData = Arc<RwLock<HashMap<uuid::Uuid, Arc<RwLock<Session>>>>>;

#[derive(Default, Clone)]
pub struct SessionStore {
    sessions: SessionData,
}

pub struct SessionInstance {
    session: Arc<RwLock<Session>>,
    sessions: SessionData,
}

impl SessionInstance {
    pub fn session(&self) -> &Arc<RwLock<Session>> {
        &self.session
    }
}

pub struct WithSession<T: Reply> {
    reply: T,
    set_cookie: Option<String>,
}

impl<T: Reply> WithSession<T> {
    pub async fn new(reply: T, instance: SessionInstance) -> Self {
        let id = instance.session.read().await.id;

        let set_cookie = if !instance.sessions.read().await.contains_key(&id) {
            let mut sessions = instance.sessions.write().await;

            if sessions.insert(id, instance.session.clone()).is_none() {
                Some(id.to_string())
            } else {
                // Not the same ID, another request set the cookie first
                None
            }
        } else {
            // Already have an ID, no need for more locking
            None
        };

        Self { reply, set_cookie }
    }
}

impl<T: Reply> Reply for WithSession<T> {
    fn into_response(self) -> warp::reply::Response {
        let mut inner = self.reply.into_response();

        if let Some(cookie_value) = self.set_cookie {
            // TODO: Other cookie options?
            inner.headers_mut().insert(
                "Set-Cookie",
                cookie::Cookie::build(COOKIE_NAME, cookie_value)
                    .finish()
                    .to_string()
                    .try_into()
                    .unwrap(),
            );
        }

        inner
    }
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request(
        &self,
    ) -> impl Filter<Extract = (SessionInstance,), Error = Rejection> + Clone + 'static {
        let sessions = self.sessions.clone();

        warp::any()
            .and(warp::any().map(move || sessions.clone()))
            .and(warp::cookie::optional(COOKIE_NAME))
            .and_then(
                |sessions: SessionData, sid_cookie: Option<String>| async move {
                    match sid_cookie
                        .and_then(|cookie_value| uuid::Uuid::parse_str(&cookie_value).ok())
                    {
                        Some(sid) => {
                            // Get the existing session
                            let session = sessions.read().await.get(&sid).cloned();

                            // Create if the ID is not found
                            let session = if let Some(session) = session {
                                session
                            } else {
                                Arc::new(RwLock::new(Session::default()))
                            };

                            Ok::<_, Rejection>(SessionInstance {
                                session,
                                sessions: sessions.clone(),
                            })
                        }
                        None => {
                            // No session yet, create one
                            Ok::<_, Rejection>(SessionInstance {
                                session: Arc::new(RwLock::new(Session::default())),
                                sessions: sessions.clone(),
                            })
                        }
                    }
                },
            )
    }
}

pub async fn reply_session<T: Reply>(
    reply: T,
    session: SessionInstance,
) -> Result<WithSession<T>, Rejection> {
    Ok(WithSession::new(reply, session).await)
}
