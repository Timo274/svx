//! Short Authentication String (SAS).
//!
//! Once the receiver has shared its ephemeral age recipient string via
//! the relay, both sides independently hash the recipient string plus
//! the transfer code and derive a short, human-pronounceable string.
//! Operators compare the SAS out-of-band (voice, Signal, DM) before
//! approving the transfer — this detects a malicious relay that has
//! substituted the recipient mid-flight. Preimage-resistance of SHA-256
//! means a relay cannot forge a recipient string whose SAS matches the
//! genuine one.
//!
//! 32 bits of SAS entropy is sufficient: a MITM gets a single chance at
//! the live transfer, a wrong guess aborts the transfer and burns the
//! transfer code.

use sha2::{Digest, Sha256};

use crate::wordlist::encode_words;

/// Compute the 4-word SAS for a transfer session.
///
/// Both the receiver and the sender call this with the recipient string
/// they have locally — if a relay has tampered with it in transit the
/// two SAS values will differ and the operators will notice when they
/// read it to each other out of band.
pub fn sas_words(recipient: &str, code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"svx-sas-v1\n");
    hasher.update(recipient.as_bytes());
    hasher.update(b"\n");
    hasher.update(code.as_bytes());
    let digest = hasher.finalize();
    encode_words(&digest[..4])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sas_is_deterministic() {
        let a = sas_words("age1abcdef", "42-eagle-harp");
        let b = sas_words("age1abcdef", "42-eagle-harp");
        assert_eq!(a, b);
        assert_eq!(a.split('-').count(), 4);
    }

    #[test]
    fn sas_changes_with_recipient_or_code() {
        let base = sas_words("age1abcdef", "42-eagle-harp");
        assert_ne!(base, sas_words("age1abcxxx", "42-eagle-harp"));
        assert_ne!(base, sas_words("age1abcdef", "43-eagle-harp"));
    }
}
