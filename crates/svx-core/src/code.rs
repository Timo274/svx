//! Transfer codes: short, human-readable room identifiers issued by the
//! relay to pair a receiver and a sender.
//!
//! Format: `<NNN>-word-word` where NNN is a 1-3 digit numeric prefix.
//! The numeric prefix is only decorative; all entropy lives in the two
//! words (2 × 256 = 65 536 combinations). The relay additionally enforces
//! a 10-minute TTL and rejects duplicate codes, so brute-force guessing a
//! live code requires ~65k attempts within 600 seconds against a rate-
//! limited endpoint. For validator-level threat models this is
//! sufficient; operators who want stronger guarantees should always
//! verify the SAS out-of-band (which is the primary MITM defense).

use rand::{Rng, RngCore};

use crate::wordlist::WORDS;

/// Generate a fresh transfer code like `42-eagle-harp`.
pub fn generate() -> String {
    let mut rng = rand::thread_rng();
    let num: u16 = rng.gen_range(10..1000);
    let mut idx = [0u8; 2];
    rng.fill_bytes(&mut idx);
    format!(
        "{}-{}-{}",
        num, WORDS[idx[0] as usize], WORDS[idx[1] as usize]
    )
}

/// Lightweight syntactic validation. The relay performs authoritative
/// checks, but clients can fail fast on obvious typos.
pub fn is_well_formed(code: &str) -> bool {
    let parts: Vec<&str> = code.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    if parts[0].parse::<u16>().is_err() {
        return false;
    }
    parts[1].chars().all(|c| c.is_ascii_alphabetic())
        && parts[2].chars().all(|c| c.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_codes_are_well_formed() {
        for _ in 0..50 {
            let c = generate();
            assert!(is_well_formed(&c), "code {c} failed validation");
        }
    }

    #[test]
    fn malformed_codes_rejected() {
        assert!(!is_well_formed(""));
        assert!(!is_well_formed("abc"));
        assert!(!is_well_formed("42-eagle"));
        assert!(!is_well_formed("foo-eagle-harp"));
        assert!(!is_well_formed("42-eag1e-harp"));
    }
}
