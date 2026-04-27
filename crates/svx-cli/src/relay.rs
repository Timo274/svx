//! WebSocket client for the svx rendezvous relay.
//!
//! The CLI talks JSON-over-WebSocket with the relay. This module hides
//! the tokio-tungstenite plumbing behind a small state machine:
//!
//! - `connect` opens the socket;
//! - `host` sends a `ClientMessage::Host` and awaits a `Welcome`;
//! - `join` sends a `ClientMessage::Join` and awaits a `Paired`;
//! - `recv_payload` / `send_payload` move opaque strings around;
//! - `done` signals a clean end-of-session.

use anyhow::{anyhow, bail};
use futures_util::{SinkExt, StreamExt};
use svx_core::{ClientMessage, ServerMessage, PROTOCOL_VERSION};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message as WsMessage, MaybeTlsStream, WebSocketStream,
};
use url::Url;

pub struct Session {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

pub async fn connect(relay: &str) -> anyhow::Result<Session> {
    let base = Url::parse(relay)?;
    let ws_url = match base.scheme() {
        "ws" | "wss" => base,
        "http" => {
            let mut u = base.clone();
            u.set_scheme("ws").map_err(|_| anyhow!("invalid scheme"))?;
            u
        }
        "https" => {
            let mut u = base.clone();
            u.set_scheme("wss").map_err(|_| anyhow!("invalid scheme"))?;
            u
        }
        other => bail!("unsupported relay scheme: {other}"),
    };
    let ws_url = {
        let mut u = ws_url;
        if u.path().is_empty() || u.path() == "/" {
            u.set_path("/ws");
        }
        u
    };
    let (ws, _resp) = connect_async(ws_url.as_str()).await?;
    Ok(Session { ws })
}

impl Session {
    async fn send(&mut self, msg: ClientMessage) -> anyhow::Result<()> {
        let body = serde_json::to_string(&msg)?;
        self.ws.send(WsMessage::Text(body)).await?;
        Ok(())
    }

    async fn recv(&mut self) -> anyhow::Result<ServerMessage> {
        loop {
            let frame = self
                .ws
                .next()
                .await
                .ok_or_else(|| anyhow!("relay connection closed"))??;
            match frame {
                WsMessage::Text(t) => {
                    let parsed: ServerMessage = serde_json::from_str(&t)?;
                    return Ok(parsed);
                }
                WsMessage::Binary(_) => bail!("relay sent unexpected binary frame"),
                WsMessage::Ping(_) | WsMessage::Pong(_) | WsMessage::Frame(_) => continue,
                WsMessage::Close(_) => bail!("relay closed connection"),
            }
        }
    }

    /// Open a new room as the receiver. Returns the transfer code.
    pub async fn host(&mut self, recipient: String) -> anyhow::Result<String> {
        self.send(ClientMessage::Host {
            version: PROTOCOL_VERSION,
            recipient,
        })
        .await?;
        match self.recv().await? {
            ServerMessage::Welcome { code, version: _ } => Ok(code),
            ServerMessage::Error { message } => bail!("relay error: {message}"),
            other => bail!("unexpected server reply: {other:?}"),
        }
    }

    /// Block until the peer (sender) joins our room. Receiver-side.
    pub async fn await_peer_joined(&mut self) -> anyhow::Result<()> {
        match self.recv().await? {
            ServerMessage::Paired { .. } => Ok(()),
            ServerMessage::Error { message } => bail!("relay error: {message}"),
            ServerMessage::PeerGone => bail!("peer disconnected before pairing"),
            other => bail!("unexpected server reply: {other:?}"),
        }
    }

    /// Join an existing room as the sender. Returns the host's
    /// recipient string for local encryption.
    pub async fn join(&mut self, code: String) -> anyhow::Result<String> {
        self.send(ClientMessage::Join {
            version: PROTOCOL_VERSION,
            code,
        })
        .await?;
        match self.recv().await? {
            ServerMessage::Paired { recipient } => Ok(recipient),
            ServerMessage::Error { message } => bail!("relay error: {message}"),
            other => bail!("unexpected server reply: {other:?}"),
        }
    }

    pub async fn send_payload(&mut self, payload: String) -> anyhow::Result<()> {
        self.send(ClientMessage::Payload { payload }).await
    }

    pub async fn recv_payload(&mut self) -> anyhow::Result<String> {
        loop {
            match self.recv().await? {
                ServerMessage::Payload { payload } => return Ok(payload),
                ServerMessage::Paired { .. } => continue, // duplicate notice, ignore
                ServerMessage::Error { message } => bail!("relay error: {message}"),
                ServerMessage::PeerGone => bail!("peer disconnected before sending payload"),
                other => bail!("unexpected server reply: {other:?}"),
            }
        }
    }

    pub async fn done(&mut self) {
        let _ = self.send(ClientMessage::Done).await;
        let _ = self.ws.close(None).await;
    }
}
