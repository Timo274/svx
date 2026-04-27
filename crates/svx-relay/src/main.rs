//! svx-relay — rendezvous server for svx transfers.
//!
//! The relay is intentionally minimal: it pairs a receiver (the first
//! client in a room) with a sender (second client) and forwards opaque
//! JSON messages between them. The server never sees plaintext identity
//! material — all encryption happens client-side via age(X25519).
//!
//! Why a relay at all? Solana validators commonly run on machines behind
//! asymmetric firewalls or NAT where direct TCP is awkward; a hosted
//! rendezvous avoids any networking knob-turning on the operator's
//! servers. Where direct connectivity is available, operators can run
//! their own relay locally and point the CLI at it.

use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, State,
    },
    response::IntoResponse,
    routing::{any, get},
    Json, Router,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use svx_core::{ClientMessage, ServerMessage, PROTOCOL_VERSION};
use tokio::sync::{mpsc, Mutex};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{debug, info, warn};

mod rooms;
use rooms::{Role, Room, Rooms};

#[derive(Clone)]
struct AppState {
    rooms: Arc<Rooms>,
    config: Arc<Config>,
}

#[derive(Clone, Debug)]
struct Config {
    /// Hard cap on the size of a single payload frame we will forward.
    max_payload_bytes: usize,
    /// Idle TTL after which a waiting room is garbage-collected.
    room_ttl: Duration,
    /// Maximum time a room can live end-to-end (prevents stragglers).
    room_lifetime: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // 256 KiB is far larger than any realistic validator identity
            // envelope (<1 KiB armored) but leaves headroom for
            // experimenting with batched transfers.
            max_payload_bytes: 256 * 1024,
            room_ttl: Duration::from_secs(600),
            room_lifetime: Duration::from_secs(30 * 60),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "svx_relay=info,tower_http=info".into()),
        )
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let state = AppState {
        rooms: Arc::new(Rooms::default()),
        config: Arc::new(Config::default()),
    };

    // Background sweeper: evict stale rooms.
    {
        let rooms = state.rooms.clone();
        let ttl = state.config.room_ttl;
        let lifetime = state.config.room_lifetime;
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(30));
            loop {
                tick.tick().await;
                rooms.sweep(ttl, lifetime);
            }
        });
    }

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/ws", any(ws_handler))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    info!("svx-relay listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

async fn root() -> impl IntoResponse {
    Json(serde_json::json!({
        "service": "svx-relay",
        "protocol_version": PROTOCOL_VERSION,
        "docs": "https://github.com/Timo274/svx",
    }))
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "ok": true,
        "active_rooms": state.rooms.len(),
        "protocol_version": PROTOCOL_VERSION,
    }))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, peer, state))
}

async fn handle_socket(socket: WebSocket, peer: SocketAddr, state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    // Buffered channel for messages the room wants us to forward to this client.
    // The writer task owns the receiver; every other task (main loop, room)
    // holds a clone of `writer_tx`. When all senders are dropped at end of
    // scope, `rx.recv()` yields None and the writer task exits cleanly.
    let (writer_tx, mut rx) = mpsc::channel::<ServerMessage>(16);

    let writer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let body = serde_json::to_string(&msg).unwrap_or_else(|_| "{}".into());
            if ws_tx.send(Message::Text(body)).await.is_err() {
                break;
            }
            if matches!(msg, ServerMessage::PeerGone | ServerMessage::Error { .. }) {
                // Drain and close politely.
                let _ = ws_tx.close().await;
                break;
            }
        }
    });

    let session_started = Instant::now();
    let mut room_state: Option<Arc<Mutex<Room>>> = None;
    let mut code: Option<String> = None;
    let mut role: Option<Role> = None;

    while let Some(msg) = ws_rx.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                debug!(?e, "ws recv error");
                break;
            }
        };
        let text = match msg {
            Message::Text(t) => t,
            Message::Binary(_) => {
                let _ = writer_tx
                    .send(ServerMessage::Error {
                        message: "binary frames are not supported".into(),
                    })
                    .await;
                break;
            }
            Message::Ping(p) => {
                // axum sends Pong automatically, but we also tolerate manual pings.
                debug!(?p, "ping");
                continue;
            }
            Message::Pong(_) | Message::Close(_) => break,
        };

        if text.len() > state.config.max_payload_bytes {
            let _ = writer_tx
                .send(ServerMessage::Error {
                    message: "message exceeds max size".into(),
                })
                .await;
            break;
        }

        let client_msg: ClientMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                let _ = writer_tx
                    .send(ServerMessage::Error {
                        message: format!("malformed message: {e}"),
                    })
                    .await;
                break;
            }
        };

        match client_msg {
            ClientMessage::Host { version, recipient } if role.is_none() => {
                if version != PROTOCOL_VERSION {
                    let _ = writer_tx
                        .send(ServerMessage::Error {
                            message: "protocol version mismatch".into(),
                        })
                        .await;
                    break;
                }
                let (new_code, room) = match state.rooms.host(recipient, writer_tx.clone()) {
                    Ok(x) => x,
                    Err(e) => {
                        warn!(%peer, error = %e, "could not allocate room");
                        let _ = writer_tx
                            .send(ServerMessage::Error {
                                message: e.to_string(),
                            })
                            .await;
                        break;
                    }
                };
                info!(%peer, %new_code, "host opened room");
                let _ = writer_tx
                    .send(ServerMessage::Welcome {
                        code: new_code.clone(),
                        version: PROTOCOL_VERSION,
                    })
                    .await;
                code = Some(new_code);
                role = Some(Role::Host);
                room_state = Some(room);
            }
            ClientMessage::Join { version, code: c } if role.is_none() => {
                if version != PROTOCOL_VERSION {
                    let _ = writer_tx
                        .send(ServerMessage::Error {
                            message: "protocol version mismatch".into(),
                        })
                        .await;
                    break;
                }
                match state.rooms.join(&c, writer_tx.clone()).await {
                    Ok(room) => {
                        info!(%peer, %c, "guest joined room");
                        // Notify this client of the host's recipient.
                        let host_recipient = { room.lock().await.host_recipient.clone() };
                        let _ = writer_tx
                            .send(ServerMessage::Paired {
                                recipient: host_recipient,
                            })
                            .await;
                        // Notify the host that the guest has arrived. We do
                        // not forward a recipient from the guest (the guest
                        // is the sender; it encrypts to the host).
                        if let Some(host_tx) = room.lock().await.host_tx.clone() {
                            let _ = host_tx
                                .send(ServerMessage::Paired {
                                    recipient: String::new(),
                                })
                                .await;
                        }
                        code = Some(c);
                        role = Some(Role::Guest);
                        room_state = Some(room);
                    }
                    Err(e) => {
                        warn!(%peer, %c, error = %e, "join rejected");
                        let _ = writer_tx
                            .send(ServerMessage::Error {
                                message: e.to_string(),
                            })
                            .await;
                        break;
                    }
                }
            }
            ClientMessage::Payload { payload } => {
                let (Some(room), Some(my_role)) = (room_state.as_ref(), role) else {
                    let _ = writer_tx
                        .send(ServerMessage::Error {
                            message: "not in a room".into(),
                        })
                        .await;
                    break;
                };
                let peer_tx = {
                    let g = room.lock().await;
                    match my_role {
                        Role::Host => g.guest_tx.clone(),
                        Role::Guest => g.host_tx.clone(),
                    }
                };
                match peer_tx {
                    Some(tx) => {
                        let _ = tx.send(ServerMessage::Payload { payload }).await;
                    }
                    None => {
                        let _ = writer_tx
                            .send(ServerMessage::Error {
                                message: "peer has not joined yet".into(),
                            })
                            .await;
                    }
                }
            }
            ClientMessage::Done => {
                break;
            }
            _ => {
                let _ = writer_tx
                    .send(ServerMessage::Error {
                        message: "unexpected message for current state".into(),
                    })
                    .await;
                break;
            }
        }

        if session_started.elapsed() > state.config.room_lifetime {
            let _ = writer_tx
                .send(ServerMessage::Error {
                    message: "session lifetime exceeded".into(),
                })
                .await;
            break;
        }
    }

    // Tear down on disconnect.
    if let (Some(room), Some(c), Some(my_role)) = (room_state, code, role) {
        state.rooms.leave(&c, my_role, &room).await;
    }
    drop(writer_tx);
    let _ = writer.await;
}
