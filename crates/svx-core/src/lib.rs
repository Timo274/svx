//! Core crypto and protocol types for svx (Solana Validator eXchange).
//!
//! This crate is `no_io`: it exposes pure functions and types for:
//! - parsing and sanity-checking Solana validator identity keypairs,
//! - generating human-friendly one-time transfer codes,
//! - computing a Short Authentication String (SAS) used by operators to
//!   verify out-of-band that no man-in-the-middle has substituted keys,
//! - end-to-end encrypting the identity payload via `age` (X25519),
//! - signing a lightweight attestation with the validator identity so the
//!   receiver can verify the transferred key is genuine.
//!
//! The relay server and CLI both depend on this crate; the relay uses only
//! the [`protocol`] module (it never sees plaintext identity material).

pub mod code;
pub mod crypto;
pub mod identity;
pub mod protocol;
pub mod sas;
pub mod wordlist;

pub use crypto::{
    decrypt_payload, encrypt_payload, generate_recipient, DecryptedMeta, EphemeralRecipient,
    SealedPayload,
};
pub use identity::{IdentityKeypair, IdentityPubkey};
pub use protocol::{ClientMessage, ServerMessage};
pub use sas::sas_words;

/// Protocol version. Bumped on incompatible relay changes.
pub const PROTOCOL_VERSION: u32 = 1;

/// Default public relay host (can be overridden via CLI flag or env).
pub const DEFAULT_RELAY_URL: &str = "wss://svx-relay.fly.dev";

/// Maximum identity file size we are willing to accept (defensive cap).
/// Real validator identity keypairs are 64 bytes serialized as a JSON array
/// of ~200 characters; we still allow 8 KiB for future formats / comments.
pub const MAX_IDENTITY_BYTES: usize = 8 * 1024;
