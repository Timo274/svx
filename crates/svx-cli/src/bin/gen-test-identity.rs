//! Tiny helper used by tests and demos to write a random Solana-format
//! validator identity file. Intentionally left out of the main CLI
//! surface so operators cannot accidentally mint identities in
//! production.

use std::path::PathBuf;

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let out: PathBuf = args
        .get(1)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("usage: gen-test-identity <output-path>"))?;
    let sk = SigningKey::generate(&mut OsRng);
    let mut bytes = [0u8; 64];
    bytes[..32].copy_from_slice(sk.as_bytes());
    bytes[32..].copy_from_slice(sk.verifying_key().as_bytes());
    let json = serde_json::to_vec(&bytes.as_slice())?;
    std::fs::write(&out, json)?;
    println!("wrote {}", out.display());
    println!(
        "pubkey: {}",
        bs58::encode(sk.verifying_key().as_bytes()).into_string()
    );
    Ok(())
}
