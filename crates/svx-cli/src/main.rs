//! svx — secure, end-to-end-encrypted transfer of a Solana validator
//! identity between two servers.
//!
//! The CLI is a thin user-facing layer on top of [`svx_core`] and a
//! WebSocket client against the rendezvous relay. It exposes three
//! commands validator operators actually run:
//!
//! - `svx receive --out /path/to/validator-keypair.json` on the machine
//!   that will host the identity next; prints a transfer code to share
//!   with the sender operator.
//! - `svx send --identity /path/to/validator-keypair.json --code XX-...`
//!   on the machine that currently holds the identity; encrypts and
//!   ships it to the receiver.
//! - `svx hot-swap` prints the canonical, no-downtime identity
//!   transition sequence recommended for staked validators.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod io;
mod relay;
mod tui;

use io::write_identity_file;
use tui::{banner, confirm_sas, error_line, info_line, prompt_sas_from_peer, section};

#[derive(Parser, Debug)]
#[command(
    name = "svx",
    about = "Securely transfer a Solana validator identity between servers",
    version
)]
struct Cli {
    /// Public relay URL. Override when running your own relay.
    #[arg(long, env = "SVX_RELAY", default_value = svx_core::DEFAULT_RELAY_URL)]
    relay: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Wait on this machine for an inbound transfer and write the
    /// decrypted identity to disk.
    Receive(ReceiveArgs),
    /// Read an identity file on this machine and send it to the holder
    /// of the given transfer code.
    Send(SendArgs),
    /// Print the canonical identity transition sequence (no network calls).
    HotSwap,
    /// Inspect an identity file locally without sending it anywhere.
    Inspect {
        #[arg(value_name = "PATH")]
        path: PathBuf,
    },
}

#[derive(clap::Args, Debug)]
struct ReceiveArgs {
    /// Where to write the received identity file on success. Parent
    /// directory must exist; the file is created with mode 0600.
    #[arg(long, short = 'o', value_name = "PATH")]
    out: PathBuf,
    /// Skip the interactive SAS confirmation. Not recommended outside of
    /// scripted tests on trusted networks.
    #[arg(long, default_value_t = false)]
    unsafe_no_sas: bool,
}

#[derive(clap::Args, Debug)]
struct SendArgs {
    /// Path to the Solana validator identity file (e.g.
    /// `/etc/solana/validator-keypair.json`).
    #[arg(long, short = 'i', value_name = "PATH")]
    identity: PathBuf,
    /// Transfer code printed by `svx receive` on the other machine.
    #[arg(long, short = 'c', value_name = "CODE")]
    code: String,
    /// Optional human-readable label attached to the envelope
    /// (metadata only — never sensitive).
    #[arg(long)]
    label: Option<String>,
    /// Skip the interactive SAS confirmation. Not recommended outside of
    /// scripted tests on trusted networks.
    #[arg(long, default_value_t = false)]
    unsafe_no_sas: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Install the default rustls crypto provider so WebSocket/TLS works
    // regardless of which feature set tokio-tungstenite pulled in.
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Receive(args) => cmd_receive(&cli.relay, args).await,
        Command::Send(args) => cmd_send(&cli.relay, args).await,
        Command::HotSwap => {
            print_hot_swap_guide();
            Ok(())
        }
        Command::Inspect { path } => cmd_inspect(&path),
    }
}

async fn cmd_receive(relay_url: &str, args: ReceiveArgs) -> anyhow::Result<()> {
    banner();
    section("receive: wait for an inbound validator identity");

    // Fail fast if the destination is already occupied.
    if args.out.exists() {
        anyhow::bail!(
            "refusing to overwrite existing file at {}. Move it aside first.",
            args.out.display()
        );
    }
    if let Some(parent) = args.out.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            anyhow::bail!("parent directory does not exist: {}", parent.display());
        }
    }

    let recipient = svx_core::generate_recipient();
    info_line(&format!(
        "generated ephemeral recipient: {}",
        recipient.recipient_str
    ));

    let mut session = relay::connect(relay_url).await?;
    let code = session.host(recipient.recipient_str.clone()).await?;
    let sas = svx_core::sas_words(&recipient.recipient_str, &code);

    section(&format!(
        "transfer code: {}",
        console::style(&code).bold().cyan()
    ));
    info_line("share this code with the sender operator through a trusted channel");
    info_line("(same person + same laptop is fine; across operators use Signal/SMS/voice)");
    info_line(&format!(
        "expected SAS (you will read this aloud): {}",
        console::style(&sas).bold().yellow()
    ));

    // Wait for the sender to join and to deliver the ciphertext.
    info_line("waiting for sender to join…");
    session.await_peer_joined().await?;
    info_line("sender joined. awaiting encrypted payload…");

    if !args.unsafe_no_sas && !confirm_sas(&sas) {
        error_line("SAS not confirmed — aborting transfer");
        return Err(anyhow::anyhow!("SAS check failed"));
    }

    let ciphertext = session.recv_payload().await?;
    let sealed = svx_core::SealedPayload {
        bytes: ciphertext.into_bytes(),
    };
    let (identity, meta) = svx_core::decrypt_payload(
        &sealed,
        &recipient.identity_str,
        &code,
        &recipient.recipient_str,
    )?;

    write_identity_file(&args.out, &identity.to_solana_json())?;
    session.done().await;

    section("transfer complete");
    info_line(&format!("wrote identity to {}", args.out.display()));
    info_line(&format!("pubkey: {}", identity.pubkey()));
    if let Some(label) = meta.label {
        info_line(&format!("label:  {label}"));
    }
    info_line("next steps: run `svx hot-swap` for the no-downtime transition sequence.");
    Ok(())
}

async fn cmd_send(relay_url: &str, args: SendArgs) -> anyhow::Result<()> {
    banner();
    section("send: ship this server's validator identity to a waiting peer");

    let raw = std::fs::read(&args.identity)?;
    let identity = svx_core::IdentityKeypair::from_solana_json(&raw)
        .map_err(|e| anyhow::anyhow!("failed to load identity file: {e}"))?;
    info_line(&format!("loaded identity for pubkey {}", identity.pubkey()));

    let mut session = relay::connect(relay_url).await?;
    let peer_recipient = session.join(args.code.clone()).await?;
    let sas = svx_core::sas_words(&peer_recipient, &args.code);
    info_line(&format!("peer recipient: {}", peer_recipient));
    info_line(&format!(
        "computed SAS (you will hear this from the receiver): {}",
        console::style(&sas).bold().yellow()
    ));

    if !args.unsafe_no_sas && !prompt_sas_from_peer(&sas) {
        error_line("SAS mismatch — aborting transfer");
        return Err(anyhow::anyhow!("SAS check failed"));
    }

    let sealed = svx_core::encrypt_payload(
        &identity,
        &peer_recipient,
        &args.code,
        args.label.as_deref(),
    )?;
    let text = String::from_utf8(sealed.bytes)?;
    session.send_payload(text).await?;
    session.done().await;

    section("transfer complete");
    info_line("the receiver has a copy of the identity. the file on this server is UNCHANGED.");
    info_line("next step: switch the active identity using `svx hot-swap` as a reference.");
    Ok(())
}

fn cmd_inspect(path: &std::path::Path) -> anyhow::Result<()> {
    let raw = std::fs::read(path)?;
    let identity = svx_core::IdentityKeypair::from_solana_json(&raw)?;
    println!("{}", identity.pubkey());
    Ok(())
}

fn print_hot_swap_guide() {
    let text = r#"
Solana validator identity — zero-downtime transition checklist
==============================================================

Goal: move a staked validator identity from server A (currently voting)
to server B with no missed votes and no double-signing window.

Prerequisites (do these BEFORE the transfer):
  • Both servers are fully synced (`agave-validator --ledger ... wait-for-restart-window` on A).
  • Both servers have an *unfunded / junk* identity keypair on disk.
  • Both servers are configured with the same vote account.
  • You can SSH between A and B.

Transition:
  1. On A: set unstaked identity so A stops voting as the real key.
       solana-validator -l /mnt/ledger set-identity /home/sol/unfunded-A.json

  2. Use svx to move the identity keypair to B (this tool):
       on B:  svx receive --out /home/sol/validator-keypair.json
       on A:  svx send --identity /home/sol/validator-keypair.json --code <CODE>

  3. On B: set the real identity live, WITHOUT restarting:
       solana-validator -l /mnt/ledger set-identity \
         --require-tower /home/sol/validator-keypair.json

  4. Verify on B that `solana-validator monitor` reports the new identity
     and that votes are landing.

  5. On A: DELETE the now-duplicate keypair file once B is happily voting.
       shred -u /home/sol/validator-keypair.json

Tower file: `svx` transfers the *identity* only. If you want to preserve
tower state across the hot-swap, copy the tower file out-of-band
(`scp -3 A:/mnt/ledger/tower-*.bin B:/mnt/ledger/`) before step 3.

Remember: two nodes must NEVER vote with the same identity at once.
Always un-stake one side before swapping.
"#;
    println!("{text}");
}
