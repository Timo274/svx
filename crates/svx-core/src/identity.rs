//! Solana validator identity keypairs.
//!
//! Solana stores keypairs on disk as a JSON array of 64 bytes where the
//! first 32 bytes are the Ed25519 seed (secret) and the last 32 bytes are
//! the public key. This module parses and validates that format and
//! exposes minimal signing helpers used to build an attestation that
//! accompanies the encrypted payload.

use std::fmt;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::MAX_IDENTITY_BYTES;

/// Raw Solana-format keypair bytes (64 bytes: seed || pubkey).
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct IdentityKeypair {
    bytes: [u8; 64],
}

/// Base58-encoded Ed25519 public key — this is what `solana-keygen pubkey`
/// returns for an identity file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityPubkey(pub String);

impl fmt::Display for IdentityPubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("identity file is too large ({0} bytes, max {max})", max = MAX_IDENTITY_BYTES)]
    TooLarge(usize),
    #[error("identity file is not a valid JSON byte array")]
    InvalidJson,
    #[error("identity file must contain exactly 64 bytes, found {0}")]
    WrongLength(usize),
    #[error("identity keypair is inconsistent: the stored pubkey does not match the seed")]
    InconsistentKeypair,
    #[error("signature verification failed")]
    BadSignature,
}

impl IdentityKeypair {
    /// Parse a Solana validator identity file (`validator-keypair.json`).
    ///
    /// Rejects files larger than [`MAX_IDENTITY_BYTES`] defensively and
    /// verifies that the embedded pubkey matches the Ed25519 derivation
    /// of the seed — catching accidentally-truncated or corrupt files
    /// before they are ever transmitted.
    pub fn from_solana_json(raw: &[u8]) -> Result<Self, IdentityError> {
        if raw.len() > MAX_IDENTITY_BYTES {
            return Err(IdentityError::TooLarge(raw.len()));
        }
        let parsed: Vec<u8> =
            serde_json::from_slice(raw).map_err(|_| IdentityError::InvalidJson)?;
        if parsed.len() != 64 {
            return Err(IdentityError::WrongLength(parsed.len()));
        }
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&parsed);

        // Derive the pubkey from the seed and cross-check.
        let seed: [u8; 32] = bytes[..32].try_into().expect("32 bytes");
        let signing_key = SigningKey::from_bytes(&seed);
        let derived_pub = signing_key.verifying_key();
        if derived_pub.as_bytes() != &bytes[32..] {
            return Err(IdentityError::InconsistentKeypair);
        }

        Ok(Self { bytes })
    }

    /// Serialize back to the canonical Solana JSON format.
    pub fn to_solana_json(&self) -> Vec<u8> {
        // serde_json::to_vec of a Vec<u8> yields a compact JSON array.
        serde_json::to_vec(&self.bytes.as_slice()).expect("serializing byte slice never fails")
    }

    /// Raw 64-byte representation (seed || pubkey). The returned slice is
    /// backed by zeroizing storage.
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.bytes
    }

    /// Base58 pubkey — matches the output of `solana-keygen pubkey`.
    pub fn pubkey(&self) -> IdentityPubkey {
        IdentityPubkey(bs58::encode(&self.bytes[32..]).into_string())
    }

    /// Sign an attestation message with the identity's Ed25519 key.
    /// The receiver uses this to verify that the sender actually holds
    /// the private key associated with the declared pubkey.
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let seed: [u8; 32] = self.bytes[..32].try_into().expect("32 bytes");
        let signing_key = SigningKey::from_bytes(&seed);
        signing_key.sign(message).to_bytes()
    }
}

impl fmt::Debug for IdentityKeypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never render secret material in Debug output.
        f.debug_struct("IdentityKeypair")
            .field("pubkey", &self.pubkey())
            .finish_non_exhaustive()
    }
}

/// Verify an attestation signature against a claimed base58 pubkey.
pub fn verify_attestation(
    pubkey: &IdentityPubkey,
    message: &[u8],
    signature: &[u8; 64],
) -> Result<(), IdentityError> {
    let raw = bs58::decode(&pubkey.0)
        .into_vec()
        .map_err(|_| IdentityError::BadSignature)?;
    if raw.len() != 32 {
        return Err(IdentityError::BadSignature);
    }
    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&raw);
    let vk = VerifyingKey::from_bytes(&pk_bytes).map_err(|_| IdentityError::BadSignature)?;
    let sig = Signature::from_bytes(signature);
    vk.verify(message, &sig)
        .map_err(|_| IdentityError::BadSignature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn make_solana_json() -> (Vec<u8>, IdentityPubkey) {
        let sk = SigningKey::generate(&mut OsRng);
        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(sk.as_bytes());
        bytes[32..].copy_from_slice(sk.verifying_key().as_bytes());
        let pk = IdentityPubkey(bs58::encode(sk.verifying_key().as_bytes()).into_string());
        (serde_json::to_vec(&bytes.as_slice()).unwrap(), pk)
    }

    #[test]
    fn parse_and_roundtrip() {
        let (raw, pk) = make_solana_json();
        let kp = IdentityKeypair::from_solana_json(&raw).unwrap();
        assert_eq!(kp.pubkey(), pk);
        let back = kp.to_solana_json();
        let kp2 = IdentityKeypair::from_solana_json(&back).unwrap();
        assert_eq!(kp.as_bytes(), kp2.as_bytes());
    }

    #[test]
    fn rejects_wrong_length() {
        let short = serde_json::to_vec(&vec![0u8; 32]).unwrap();
        let err = IdentityKeypair::from_solana_json(&short).unwrap_err();
        assert!(matches!(err, IdentityError::WrongLength(32)));
    }

    #[test]
    fn rejects_inconsistent_pubkey() {
        let (mut raw, _) = make_solana_json();
        // Flip a byte in the embedded pubkey portion to make it inconsistent.
        let s = std::str::from_utf8(&raw).unwrap().to_string();
        let mut parsed: Vec<u8> = serde_json::from_str(&s).unwrap();
        parsed[40] ^= 0xff;
        raw = serde_json::to_vec(&parsed).unwrap();
        let err = IdentityKeypair::from_solana_json(&raw).unwrap_err();
        assert!(matches!(err, IdentityError::InconsistentKeypair));
    }

    #[test]
    fn sign_and_verify() {
        let (raw, pk) = make_solana_json();
        let kp = IdentityKeypair::from_solana_json(&raw).unwrap();
        let msg = b"transfer attestation for session xyz";
        let sig = kp.sign(msg);
        verify_attestation(&pk, msg, &sig).unwrap();

        // Tamper with the message.
        let bad = b"transfer attestation for session abc";
        assert!(verify_attestation(&pk, bad, &sig).is_err());
    }

    #[test]
    fn rejects_oversized() {
        let raw = vec![b'x'; MAX_IDENTITY_BYTES + 1];
        assert!(matches!(
            IdentityKeypair::from_solana_json(&raw),
            Err(IdentityError::TooLarge(_))
        ));
    }
}
