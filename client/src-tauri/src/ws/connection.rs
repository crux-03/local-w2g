use std::time::Duration;

use futures::channel::oneshot;
use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Manager};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};
use tokio_tungstenite::{client_async_tls, tungstenite};
use tokio_tungstenite::tungstenite::Message;

use crate::core::AppState;
use crate::error::Error;
use crate::protocol::ClientMessage;

use super::dispatcher;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(3);
const RECONNECT_MIN: Duration = Duration::from_secs(1);
const RECONNECT_MAX: Duration = Duration::from_secs(30);

/// Handle given to Tauri state so commands can send messages to the server.
#[derive(Clone)]
pub struct WsHandle {
    tx: mpsc::UnboundedSender<ClientMessage>,
}

impl WsHandle {
    pub fn send(&self, msg: ClientMessage) -> crate::error::Result<()> {
        self.tx.send(msg).map_err(|_| Error::ChannelClosed)
    }
}

use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("invalid request: {0}")]
    BuildRequest(String),
    #[error("authentication rejected (status {0})")]
    Auth(u16),
    #[error("tcp connect failed: {0}")]
    Tcp(String),
    #[error("websocket handshake failed: {0}")]
    Handshake(String),
    #[error("connect task died before signaling readiness")]
    TaskDied,
}

pub fn spawn(
    url: String,
    username: String,
    pw: String,
    app: AppHandle,
) -> (WsHandle, oneshot::Receiver<Result<(), ConnectError>>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let (ready_tx, ready_rx) = oneshot::channel();
    tokio::spawn(run(url, username, pw, rx, app, Some(ready_tx)));
    (WsHandle { tx }, ready_rx)
}

async fn run(
    url: String,
    username: String,
    pw: String,
    mut outgoing: mpsc::UnboundedReceiver<ClientMessage>,
    app: AppHandle,
    mut ready: Option<oneshot::Sender<Result<(), ConnectError>>>,
) {
    tracing::trace!("entered run()");
    let mut backoff = RECONNECT_MIN;

    loop {
        tracing::trace!(%url, "connecting");

        // --- build request ---------------------------------------------------
        let request = match build_request(&url, &username, &pw) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(error = %e, "failed to build ws request");
                if let Some(tx) = ready.take() {
                    let _ = tx.send(Err(ConnectError::BuildRequest(e.to_string())));
                }
                return; // bad input — never retryable
            }
        };

        let uri = request.uri().clone();
        let host = match uri.host() {
            Some(h) => h.to_string(),
            None => {
                tracing::error!("ws url has no host");
                if let Some(tx) = ready.take() {
                    let _ = tx.send(Err(ConnectError::BuildRequest("url has no host".into())));
                }
                return;
            }
        };
        let port = uri.port_u16().unwrap_or_else(|| match uri.scheme_str() {
            Some("wss") => 443,
            _ => 80,
        });

        // --- tcp connect -----------------------------------------------------
        let tcp = match TcpStream::connect((host.as_str(), port)).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "tcp connect failed");
                // First attempt: surface the error to the UI immediately rather
                // than silently retrying — a wrong URL shouldn't look like a hang.
                if let Some(tx) = ready.take() {
                    let _ = tx.send(Err(ConnectError::Tcp(e.to_string())));
                    return;
                }
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(RECONNECT_MAX);
                continue;
            }
        };

        if let Err(e) = tcp.set_nodelay(true) {
            tracing::warn!(error = %e, "failed to set TCP_NODELAY");
        }

        // --- ws handshake ----------------------------------------------------
        match client_async_tls(request, tcp).await {
            Ok((ws, _)) => {
                tracing::info!("connected");
                backoff = RECONNECT_MIN;

                // First successful handshake: unblock the frontend.
                if let Some(tx) = ready.take() {
                    let _ = tx.send(Ok(()));
                }

                match handle_connection(ws, &mut outgoing, &app).await {
                    Ok(()) => tracing::info!("connection closed cleanly"),
                    Err(e) => tracing::warn!(error = %e, "connection lost"),
                }
            }
            Err(e) => {
                // 401/403 means the credentials are wrong — looping won't help.
                if let tungstenite::Error::Http(resp) = &e {
                    let status = resp.status().as_u16();
                    if matches!(status, 401 | 403) {
                        tracing::error!(%status, "authentication rejected");
                        if let Some(tx) = ready.take() {
                            let _ = tx.send(Err(ConnectError::Auth(status)));
                        }
                        return;
                    }
                }

                tracing::warn!(error = %e, "handshake failed");
                if let Some(tx) = ready.take() {
                    // First attempt failed before we ever connected — bail out
                    // rather than retry behind the user's back.
                    let _ = tx.send(Err(ConnectError::Handshake(e.to_string())));
                    return;
                }
            }
        }

        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(RECONNECT_MAX);
    }
}

fn build_request(
    url: &str,
    username: &str,
    pw: &str,
) -> Result<
    tokio_tungstenite::tungstenite::http::Request<()>,
    Box<dyn std::error::Error + Send + Sync>,
> {
    let mut req = url.into_client_request()?;
    let headers = req.headers_mut();
    headers.insert("X-Access-Key", HeaderValue::from_str(pw)?);
    headers.insert("X-Username", HeaderValue::from_str(username)?);
    tracing::info!("Connection Headers: {:?}", headers);
    Ok(req)
}

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn handle_connection(
    ws: WsStream,
    outgoing: &mut mpsc::UnboundedReceiver<ClientMessage>,
    app: &AppHandle,
) -> crate::error::Result<()> {
    let state = app.state::<AppState>();
    tracing::info!("constructing websocket sinks");
    let (mut sink, mut stream) = ws.split();

    tracing::info!("create heartbeat");
    let mut heartbeat = interval(HEARTBEAT_INTERVAL);
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);
    heartbeat.tick().await; // consume the immediate first tick

    loop {
        tokio::select! {
            incoming = stream.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        dispatcher::handle(&text, app).await;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        sink.send(Message::Pong(data)).await?;
                    }
                    Some(Ok(Message::Close(_))) | None => return Ok(()),
                    Some(Ok(_)) => {} // Binary / Pong / Frame — ignore
                    Some(Err(e)) => return Err(e.into()),
                }
            }

            client_msg = outgoing.recv() => {
                match client_msg {
                    Some(msg) => send_json(&mut sink, &msg).await?,
                    None => return Ok(()), // app shutting down
                }
            }

            _ = heartbeat.tick() => {
                send_json(&mut sink, &ClientMessage::Heartbeat).await?;
                state.start_ping().await;
                send_json(&mut sink, &ClientMessage::Ping).await?;
            }
        }
    }
}

async fn send_json<S>(sink: &mut S, msg: &ClientMessage) -> crate::error::Result<()>
where
    S: SinkExt<Message> + Unpin,
    Error: From<<S as futures_util::Sink<Message>>::Error>,
{
    let text = serde_json::to_string(msg)?;
    sink.send(Message::Text(text.into())).await?;
    Ok(())
}
