//! Wire protocol between the relay and its clients.
//!
//! The relay is a thin rendezvous server: it pairs a receiver (first
//! client in a room) with a sender (second client) and forwards their
//! encrypted payloads back and forth. It never sees identity material.
//!
//! Messages are JSON, tagged with the `"type"` field. The relay is
//! responsible for generating transfer codes, enforcing one-writer-per-
//! room, and expiring idle rooms.

use serde::{Deserialize, Serialize};

/// Messages a client sends to the relay.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Sent by the receiver to open a new room. The relay responds with
    /// [`ServerMessage::Welcome`] containing a freshly-generated code.
    Host {
        /// Client protocol version.
        version: u32,
        /// The age recipient string this receiver wants peers to encrypt
        /// to. The relay treats it as opaque.
        recipient: String,
    },

    /// Sent by the sender to join an existing room.
    Join { version: u32, code: String },

    /// Opaque ciphertext + envelope metadata from one peer to the other.
    Payload { payload: String },

    /// Signal that the peer may release the room cleanly.
    Done,
}

/// Messages the relay sends to a client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Reply to [`ClientMessage::Host`]. The receiver now waits for a
    /// sender to join under the given code.
    Welcome {
        code: String,
        /// Server-advertised protocol version.
        version: u32,
    },

    /// Sent to the sender when [`ClientMessage::Join`] succeeds, and to
    /// the receiver when a sender arrives. Carries the peer's recipient
    /// string so senders can encrypt to it; senders receive this first
    /// before issuing [`ClientMessage::Payload`].
    Paired { recipient: String },

    /// Forwarded [`ClientMessage::Payload`] from the peer.
    Payload { payload: String },

    /// The peer has disconnected or signalled completion.
    PeerGone,

    /// The relay is rejecting the request.
    Error { message: String },
}
