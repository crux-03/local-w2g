use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::error::Error;
use crate::protocol::ClientMessage;

use super::dispatcher;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
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

pub fn spawn(url: String, username: String, pw: String, app: AppHandle) -> WsHandle {
    tracing::info!("entered spawn()");
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(run(url, username, pw, rx, app));
    WsHandle { tx }
}

async fn run(
    url: String,
    username: String,
    pw: String,
    mut outgoing: mpsc::UnboundedReceiver<ClientMessage>,
    app: AppHandle,
) {
    tracing::info!("Entered run()");
    let mut backoff = RECONNECT_MIN;

    loop {
        tracing::info!(%url, "connecting");

        // Build the request fresh each loop iteration so reconnects
        // carry the credentials too.
        let request = match build_request(&url, &username, &pw) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(error = %e, "failed to build ws request");
                return;
            }
        };

        match connect_async(request).await {
            Ok((ws, _)) => {
                tracing::info!("connected");
                backoff = RECONNECT_MIN;
                match handle_connection(ws, &mut outgoing, &app).await {
                    Ok(()) => tracing::info!("connection closed cleanly"),
                    Err(e) => tracing::warn!(error = %e, "connection lost"),
                }
            }
            Err(e) => tracing::warn!(error = %e, "connect failed"),
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
