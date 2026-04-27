//! End-to-end encryption using age (X25519 + ChaCha20-Poly1305).
//!
//! age is a modern, well-audited file-encryption format that gives us
//! authenticated asymmetric encryption with minimal ceremony. The
//! receiver generates an ephemeral age identity (X25519 private key)
//! locally, shares only the public recipient string with the sender via
//! the relay, and the sender encrypts the identity payload to that
//! recipient. The relay and any passive observer see only ciphertext.
//!
//! The payload we encrypt is a JSON envelope ([`TransferEnvelope`]) that
//! bundles the Solana validator identity keypair bytes together with a
//! self-describing attestation signed by the identity itself. This lets
//! the receiver verify after decryption that the declared pubkey really
//! matches the transferred secret key, and that the envelope was built
//! for this specific session.

use std::io::{Read, Write};

use age::{secrecy::ExposeSecret, x25519, Decryptor, Encryptor, Recipient};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::Zeroize;

use crate::identity::{verify_attestation, IdentityKeypair, IdentityPubkey};

/// Ciphertext produced by [`encrypt_payload`]. `bytes` is a standard
/// age-armored (ASCII) blob so it can be transported over text channels
/// if needed without base64 wrapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedPayload {
    pub bytes: Vec<u8>,
}

/// The plaintext envelope. Everything here is sent encrypted to the
/// receiver's ephemeral recipient.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferEnvelope {
    /// Protocol version of this envelope.
    pub version: u32,
    /// Declared base58 Ed25519 pubkey of the validator identity.
    pub identity_pubkey: IdentityPubkey,
    /// Raw 64-byte Solana keypair, base64 encoded (seed || pubkey).
    pub identity_keypair_b64: String,
    /// Operator-supplied label (e.g. "mainnet-validator"). Never sensitive.
    pub label: Option<String>,
    /// The transfer code used for this session — bound into the
    /// attestation to prevent replay across sessions.
    pub session_code: String,
    /// Receiver's age recipient string — bound into the attestation so a
    /// malicious relay cannot reroute the envelope to a different
    /// receiver mid-flight.
    pub recipient: String,
    /// Ed25519 signature (base64) over a canonical message authenticating
    /// `identity_pubkey`, `session_code`, `recipient` and the identity
    /// keypair bytes. Signed with the identity's own key.
    pub attestation_sig_b64: String,
}

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid recipient string")]
    InvalidRecipient,
    #[error("invalid identity string")]
    InvalidIdentity,
    #[error("encryption failed: {0}")]
    Encrypt(String),
    #[error("decryption failed: {0}")]
    Decrypt(String),
    #[error("envelope is malformed")]
    MalformedEnvelope,
    #[error("attestation signature does not match declared pubkey")]
    BadAttestation,
    #[error("transferred keypair is internally inconsistent")]
    InconsistentKeypair,
    #[error("envelope was not addressed to this session")]
    WrongSession,
}

/// A freshly-generated ephemeral age identity for one transfer.
pub struct EphemeralRecipient {
    pub identity_str: String,
    pub recipient_str: String,
}

/// Generate a fresh ephemeral age X25519 identity for one transfer. The
/// secret string never leaves the receiver's process memory; only the
/// recipient string is transmitted over the relay.
pub fn generate_recipient() -> EphemeralRecipient {
    let id = x25519::Identity::generate();
    let recipient_str = id.to_public().to_string();
    let identity_str = id.to_string().expose_secret().to_owned();
    EphemeralRecipient {
        identity_str,
        recipient_str,
    }
}

/// Encrypt a validator identity to the receiver's recipient string.
///
/// Builds an attestation signed by the identity keypair itself so the
/// receiver can verify after decryption that the material is genuine and
/// was destined for their specific session.
pub fn encrypt_payload(
    identity: &IdentityKeypair,
    recipient_str: &str,
    session_code: &str,
    label: Option<&str>,
) -> Result<SealedPayload, CryptoError> {
    let recipient: x25519::Recipient = recipient_str
        .parse()
        .map_err(|_| CryptoError::InvalidRecipient)?;

    let pubkey = identity.pubkey();
    let identity_keypair_b64 = base64_encode(identity.as_bytes());

    let attestation_msg =
        canonical_attestation_message(&pubkey, session_code, recipient_str, identity.as_bytes());
    let sig = identity.sign(&attestation_msg);
    let attestation_sig_b64 = base64_encode(&sig);

    let envelope = TransferEnvelope {
        version: crate::PROTOCOL_VERSION,
        identity_pubkey: pubkey,
        identity_keypair_b64,
        label: label.map(|s| s.to_owned()),
        session_code: session_code.to_owned(),
        recipient: recipient_str.to_owned(),
        attestation_sig_b64,
    };
    let mut plaintext =
        serde_json::to_vec(&envelope).map_err(|_| CryptoError::MalformedEnvelope)?;

    let recipients: Vec<&dyn Recipient> = vec![&recipient];
    let encryptor = Encryptor::with_recipients(recipients.into_iter())
        .map_err(|e| CryptoError::Encrypt(e.to_string()))?;
    let mut out = Vec::new();
    {
        let armored =
            age::armor::ArmoredWriter::wrap_output(&mut out, age::armor::Format::AsciiArmor)
                .map_err(|e| CryptoError::Encrypt(e.to_string()))?;
        let mut writer = encryptor
            .wrap_output(armored)
            .map_err(|e| CryptoError::Encrypt(e.to_string()))?;
        writer
            .write_all(&plaintext)
            .map_err(|e| CryptoError::Encrypt(e.to_string()))?;
        writer
            .finish()
            .map_err(|e| CryptoError::Encrypt(e.to_string()))?
            .finish()
            .map_err(|e| CryptoError::Encrypt(e.to_string()))?;
    }
    plaintext.zeroize();
    Ok(SealedPayload { bytes: out })
}

/// Decrypt a payload with the receiver's ephemeral identity string and
/// validate the enclosed attestation against the expected session.
///
/// Returns the recovered [`IdentityKeypair`] and the envelope metadata.
pub fn decrypt_payload(
    sealed: &SealedPayload,
    identity_str: &str,
    expected_session_code: &str,
    expected_recipient: &str,
) -> Result<(IdentityKeypair, DecryptedMeta), CryptoError> {
    let identity: x25519::Identity = identity_str
        .parse()
        .map_err(|_| CryptoError::InvalidIdentity)?;
    let decryptor = Decryptor::new(age::armor::ArmoredReader::new(&sealed.bytes[..]))
        .map_err(|e| CryptoError::Decrypt(e.to_string()))?;
    let identities: Vec<&dyn age::Identity> = vec![&identity];
    let mut reader = decryptor
        .decrypt(identities.into_iter())
        .map_err(|e| CryptoError::Decrypt(e.to_string()))?;
    let mut plaintext = Vec::new();
    reader
        .read_to_end(&mut plaintext)
        .map_err(|e| CryptoError::Decrypt(e.to_string()))?;

    let envelope: TransferEnvelope =
        serde_json::from_slice(&plaintext).map_err(|_| CryptoError::MalformedEnvelope)?;
    plaintext.zeroize();

    if envelope.session_code != expected_session_code || envelope.recipient != expected_recipient {
        return Err(CryptoError::WrongSession);
    }

    let kp_bytes = base64_decode(&envelope.identity_keypair_b64)
        .map_err(|_| CryptoError::MalformedEnvelope)?;
    let identity_kp = IdentityKeypair::from_solana_json(&kp_bytes_to_solana_json(&kp_bytes)?)
        .map_err(|_| CryptoError::InconsistentKeypair)?;

    let sig_bytes =
        base64_decode(&envelope.attestation_sig_b64).map_err(|_| CryptoError::MalformedEnvelope)?;
    if sig_bytes.len() != 64 {
        return Err(CryptoError::MalformedEnvelope);
    }
    let mut sig = [0u8; 64];
    sig.copy_from_slice(&sig_bytes);
    let attestation_msg = canonical_attestation_message(
        &envelope.identity_pubkey,
        &envelope.session_code,
        &envelope.recipient,
        identity_kp.as_bytes(),
    );
    verify_attestation(&envelope.identity_pubkey, &attestation_msg, &sig)
        .map_err(|_| CryptoError::BadAttestation)?;
    if identity_kp.pubkey() != envelope.identity_pubkey {
        return Err(CryptoError::BadAttestation);
    }

    Ok((
        identity_kp,
        DecryptedMeta {
            label: envelope.label,
            session_code: envelope.session_code,
            identity_pubkey: envelope.identity_pubkey,
        },
    ))
}

/// Metadata from a successfully decrypted envelope.
#[derive(Debug, Clone)]
pub struct DecryptedMeta {
    pub label: Option<String>,
    pub session_code: String,
    pub identity_pubkey: IdentityPubkey,
}

fn kp_bytes_to_solana_json(raw: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if raw.len() != 64 {
        return Err(CryptoError::InconsistentKeypair);
    }
    serde_json::to_vec(&raw).map_err(|_| CryptoError::MalformedEnvelope)
}

fn canonical_attestation_message(
    pubkey: &IdentityPubkey,
    session_code: &str,
    recipient: &str,
    keypair_bytes: &[u8; 64],
) -> Vec<u8> {
    // A fixed domain separator keeps this signature useless for anything
    // other than a svx transfer attestation.
    let mut msg = Vec::with_capacity(256);
    msg.extend_from_slice(b"svx-attestation-v1\n");
    msg.extend_from_slice(pubkey.0.as_bytes());
    msg.push(b'\n');
    msg.extend_from_slice(session_code.as_bytes());
    msg.push(b'\n');
    msg.extend_from_slice(recipient.as_bytes());
    msg.push(b'\n');
    // Bind the keypair bytes themselves so tampered payloads fail.
    msg.extend_from_slice(keypair_bytes);
    msg
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    STANDARD.encode(bytes)
}

fn base64_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    STANDARD.decode(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn make_identity() -> IdentityKeypair {
        let sk = SigningKey::generate(&mut OsRng);
        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(sk.as_bytes());
        bytes[32..].copy_from_slice(sk.verifying_key().as_bytes());
        let raw = serde_json::to_vec(&bytes.as_slice()).unwrap();
        IdentityKeypair::from_solana_json(&raw).unwrap()
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let r = generate_recipient();
        let identity = make_identity();
        let sealed = encrypt_payload(
            &identity,
            &r.recipient_str,
            "42-eagle-harp",
            Some("mainnet"),
        )
        .unwrap();
        let (decrypted, meta) =
            decrypt_payload(&sealed, &r.identity_str, "42-eagle-harp", &r.recipient_str).unwrap();
        assert_eq!(decrypted.pubkey(), identity.pubkey());
        assert_eq!(meta.label.as_deref(), Some("mainnet"));
    }

    #[test]
    fn wrong_session_code_rejected() {
        let r = generate_recipient();
        let identity = make_identity();
        let sealed = encrypt_payload(&identity, &r.recipient_str, "42-eagle-harp", None).unwrap();
        let err = decrypt_payload(&sealed, &r.identity_str, "99-zebra-zoom", &r.recipient_str)
            .unwrap_err();
        assert!(matches!(err, CryptoError::WrongSession));
    }

    #[test]
    fn wrong_recipient_cannot_decrypt() {
        let r_a = generate_recipient();
        let r_b = generate_recipient();
        let identity = make_identity();
        let sealed = encrypt_payload(&identity, &r_a.recipient_str, "42-eagle-harp", None).unwrap();
        // Wrong secret for the ciphertext.
        assert!(matches!(
            decrypt_payload(
                &sealed,
                &r_b.identity_str,
                "42-eagle-harp",
                &r_a.recipient_str
            ),
            Err(CryptoError::Decrypt(_))
        ));
    }
}
