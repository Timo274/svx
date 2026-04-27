//! In-memory room registry. Each room pairs one host (receiver) with one
//! guest (sender). Rooms are short-lived: evicted once both clients
//! disconnect, after 10 minutes with no guest, or after 30 minutes in
//! total. All data here is transient — restarting the relay drops all
//! in-flight sessions, which is the right default given its rendezvous-
//! only role.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use svx_core::{code, ServerMessage};
use thiserror::Error;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// First client in the room — the receiver of identity material.
    Host,
    /// Second client — the sender, holding the identity on disk.
    Guest,
}

impl core::fmt::Display for Role {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Role::Host => "host",
            Role::Guest => "guest",
        })
    }
}

/// Single rendezvous room.
pub struct Room {
    pub host_recipient: String,
    pub host_tx: Option<mpsc::Sender<ServerMessage>>,
    pub guest_tx: Option<mpsc::Sender<ServerMessage>>,
    pub created_at: Instant,
    pub guest_joined_at: Option<Instant>,
}

#[derive(Debug, Error)]
pub enum JoinError {
    #[error("transfer code is unknown or expired")]
    NoSuchCode,
    #[error("a sender has already joined this room")]
    AlreadyPaired,
}

#[derive(Debug, Error)]
pub enum HostError {
    #[error("could not allocate a unique transfer code after many attempts")]
    NoCodeAvailable,
}

#[derive(Default)]
pub struct Rooms {
    inner: DashMap<String, Arc<Mutex<Room>>>,
}

impl Rooms {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn host(
        &self,
        recipient: String,
        host_tx: mpsc::Sender<ServerMessage>,
    ) -> Result<(String, Arc<Mutex<Room>>), HostError> {
        // Retry on extremely unlikely collisions. We use DashMap's `entry`
        // API so the check-and-insert is atomic; two concurrent hosts that
        // happen to draw the same code cannot overwrite each other.
        let mut recipient = Some(recipient);
        let mut host_tx = Some(host_tx);
        for _ in 0..32 {
            let candidate = code::generate();
            match self.inner.entry(candidate.clone()) {
                Entry::Occupied(_) => continue,
                Entry::Vacant(slot) => {
                    let room = Arc::new(Mutex::new(Room {
                        host_recipient: recipient.take().expect("once"),
                        host_tx: Some(host_tx.take().expect("once")),
                        guest_tx: None,
                        created_at: Instant::now(),
                        guest_joined_at: None,
                    }));
                    slot.insert(room.clone());
                    return Ok((candidate, room));
                }
            }
        }
        Err(HostError::NoCodeAvailable)
    }

    pub async fn join(
        &self,
        code: &str,
        guest_tx: mpsc::Sender<ServerMessage>,
    ) -> Result<Arc<Mutex<Room>>, JoinError> {
        let room = self
            .inner
            .get(code)
            .ok_or(JoinError::NoSuchCode)?
            .value()
            .clone();
        let mut g = room.lock().await;
        if g.guest_tx.is_some() {
            return Err(JoinError::AlreadyPaired);
        }
        g.guest_tx = Some(guest_tx);
        g.guest_joined_at = Some(Instant::now());
        drop(g);
        Ok(room)
    }

    pub async fn leave(&self, code: &str, role: Role, room: &Arc<Mutex<Room>>) {
        let mut g = room.lock().await;
        let peer_tx = match role {
            Role::Host => {
                g.host_tx = None;
                g.guest_tx.clone()
            }
            Role::Guest => {
                g.guest_tx = None;
                g.host_tx.clone()
            }
        };
        let both_gone = g.host_tx.is_none() && g.guest_tx.is_none();
        drop(g);
        if let Some(tx) = peer_tx {
            let _ = tx.send(ServerMessage::PeerGone).await;
        }
        if both_gone {
            // Only evict the room we actually own: after a sweep-then-rehost
            // race, the code string could have been reallocated to a brand
            // new room with a different Arc, and blindly removing by key
            // would delete that innocent room out from under its host.
            self.inner
                .remove_if(code, |_, existing| Arc::ptr_eq(existing, room));
            info!(%code, "room closed");
        }
    }

    pub fn sweep(&self, idle_ttl: Duration, hard_lifetime: Duration) {
        let now = Instant::now();
        self.inner.retain(|code, room| {
            // Skip rooms that are currently being updated; we'll retry next tick.
            let Ok(g) = room.try_lock() else { return true };
            let past_hard = now.duration_since(g.created_at) > hard_lifetime;
            let idle = g.guest_joined_at.is_none() && now.duration_since(g.created_at) > idle_ttl;
            let abandoned = g.host_tx.is_none() && g.guest_tx.is_none();
            let keep = !(past_hard || idle || abandoned);
            if !keep {
                info!(%code, past_hard, idle, abandoned, "evicting stale room");
                // Notify any still-connected clients that the room expired
                // so their WebSocket handler wakes up, writes the error out,
                // and tears the TCP connection down. Without this they hang
                // on `ws_rx.next().await` forever and leak tasks.
                let expired = ServerMessage::Error {
                    message: "transfer session expired".to_string(),
                };
                if let Some(tx) = &g.host_tx {
                    if let Err(e) = tx.try_send(expired.clone()) {
                        warn!(%code, error = %e, "could not notify host of expiry");
                    }
                }
                if let Some(tx) = &g.guest_tx {
                    if let Err(e) = tx.try_send(expired) {
                        warn!(%code, error = %e, "could not notify guest of expiry");
                    }
                }
            }
            keep
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn host_allocates_unique_codes_without_overwriting() {
        let rooms = Rooms::default();
        let (tx1, _rx1) = mpsc::channel(4);
        let (tx2, _rx2) = mpsc::channel(4);
        let (c1, _r1) = rooms.host("age1a".into(), tx1).unwrap();
        let (c2, _r2) = rooms.host("age1b".into(), tx2).unwrap();
        assert_ne!(c1, c2);
        assert_eq!(rooms.len(), 2);
    }

    #[tokio::test]
    async fn sweep_notifies_host_when_room_expires() {
        let rooms = Rooms::default();
        let (host_tx, mut host_rx) = mpsc::channel(4);
        let (_code, _room) = rooms.host("age1abc".into(), host_tx).unwrap();
        // idle TTL of 0 forces immediate eviction
        tokio::time::sleep(Duration::from_millis(5)).await;
        rooms.sweep(Duration::from_nanos(1), Duration::from_secs(3600));
        let msg = host_rx
            .recv()
            .await
            .expect("host should receive an expiry notification");
        assert!(matches!(msg, ServerMessage::Error { .. }));
        assert_eq!(rooms.len(), 0);
    }

    /// Simulates the sweep-then-rehost race: a handler holds a stale Arc
    /// for code C, the room gets evicted by sweep, a fresh host claims
    /// the same C, then the stale handler calls `leave`. Leave must only
    /// remove the room if it matches the Arc the handler was tracking.
    #[tokio::test]
    async fn leave_does_not_evict_a_rehosted_room() {
        let rooms = Rooms::default();
        let (stale_tx, _stale_rx) = mpsc::channel(4);
        let (stale_code, stale_room) = rooms.host("age1old".into(), stale_tx).unwrap();

        // Pretend sweep already dropped the original entry.
        rooms.inner.remove(&stale_code);
        assert_eq!(rooms.len(), 0);

        // A brand new host happens to draw the same code.
        // Build the Room directly so we control the Arc identity — we want
        // a fresh Arc, distinct from `stale_room`.
        let (fresh_tx, _fresh_rx) = mpsc::channel::<ServerMessage>(4);
        let fresh_room = Arc::new(Mutex::new(Room {
            host_recipient: "age1new".into(),
            host_tx: Some(fresh_tx),
            guest_tx: None,
            created_at: Instant::now(),
            guest_joined_at: None,
        }));
        rooms.inner.insert(stale_code.clone(), fresh_room);
        assert_eq!(rooms.len(), 1);

        // Now the stale handler finally disconnects. Its leave must be a no-op
        // against the registry — the fresh room must survive.
        rooms.leave(&stale_code, Role::Host, &stale_room).await;
        assert_eq!(rooms.len(), 1);
    }
}
