# svx — secure Solana validator identity transfer

**svx** is a small, open-source CLI that moves a Solana validator's private
identity key from one server to another without ever exposing it to a
third party. It is designed for the real operational moment validator
operators hit regularly: replacing the primary box, migrating to a new
data center, or performing a planned failover — and wanting to do it in
five minutes, end-to-end encrypted, with a sanity check that cannot be
cheated by a malicious relay.

<p align="center"><b>
Live site: <a href="https://solana-svx.fly.dev">solana-svx.fly.dev</a> &nbsp;·&nbsp;
Public relay: <a href="https://svx-relay.fly.dev/health">svx-relay.fly.dev</a>
</b></p>

## The product in one sentence

A two-command tool — `svx receive` on the destination, `svx send` on
the source — that streams your `validator-keypair.json` between the
two hosts over a rendezvous relay, with end-to-end encryption plus a
4-word Short Authentication String that the operators verify
out-of-band.

## Target user

Solana validator operators running mainnet-beta, testnet, or SFDP
nodes who need to:

- upgrade a host and move the identity to new hardware,
- migrate across data centers / ASNs,
- run a planned failover to a pre-warmed secondary,
- or simply stop shuffling keypair files over `scp`, SSH agent-forwarding,
  password-manager uploads, or email.

It's equally useful for a **single operator** moving a key between their
own two servers and for a **team** where one engineer holds the source
server and another holds the destination.

## Why not just `scp`?

- `scp` leaks the key to anyone with root on either machine, on any
  bastion in the SSH path, or with access to the SSH private key and a
  capture of the network path.
- The operator has no cryptographic proof of *which* server they sent
  the key to — `scp` is fire-and-forget.
- There is no defense in depth against a compromised jump host or a
  typo'd destination. `svx` adds a 4-word SAS comparison that shows,
  out-of-band, that the sender and receiver are talking to each other
  and nothing in between has substituted keys.

## How it works

```
┌─────────────┐        transfer code             ┌────────────┐
│  node A     │  ───────────────────────────▶   │  node B    │
│  (source)   │                                 │ (destination)│
│             │                                 │             │
│  svx send   │◀── ciphertext (age/X25519) ──▶ │ svx receive │
└─────────────┘        via relay                 └────────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │   svx-relay     │  ← sees only opaque ciphertext
                    │ (ciphertext-only│
                    │   postman)      │
                    └─────────────────┘
```

1. The receiver runs `svx receive --out /path/to/validator-keypair.json`.
   The CLI generates an ephemeral X25519 keypair locally, opens a room
   on the relay, and prints a one-time **transfer code** like
   `371-extra-bliss` plus the expected **SAS**, a 4-word string derived
   from the ephemeral public key and the code.

2. The sender runs `svx send --identity /path/... --code 371-extra-bliss`
   on the other machine, learns the receiver's age recipient over the
   same relay, computes its own SAS, and shows it to the operator. The
   two operators compare SAS strings out-of-band (voice, Signal, DM).
   **If the SAS does not match, a MITM is happening — the transfer
   aborts.**

3. With SAS confirmed, the sender encrypts a signed envelope containing
   the identity keypair to the receiver's public key (age v1,
   X25519 + ChaCha20-Poly1305) and ships it through the relay.

4. The receiver decrypts, verifies the Ed25519 attestation against the
   declared pubkey, and writes the file atomically with mode `0600`.

5. The operator then performs the canonical no-downtime identity
   transition with `solana-validator set-identity` — `svx hot-swap`
   prints the full checklist.

## Security properties

- **Confidentiality:** age v1 authenticated encryption (X25519 +
  ChaCha20-Poly1305). The private key never leaves the receiver.
- **Authenticity:** an Ed25519 attestation signed by the validator
  identity itself is bundled into the envelope. The receiver verifies
  the declared pubkey really matches the transferred secret key.
- **MITM resistance:** 4-word SAS bound to the ephemeral recipient and
  transfer code. Preimage-resistance of SHA-256 means a relay cannot
  forge a recipient with the same SAS.
- **Replay resistance:** the envelope binds the session's transfer
  code; replaying a recorded ciphertext into a new session is rejected.
- **Damage containment:** the relay is stateless, ciphertext-only,
  with a 10-minute idle TTL and 30-minute hard lifetime per room.
  A compromised relay cannot decrypt, cannot forge SAS, and can at
  most deny service.
- **Defensive file handling:** destination files are created with
  `O_CREAT | O_EXCL` at mode `0600`; svx refuses to overwrite an
  existing file.

## Install

```bash
# requires Rust 1.85+
git clone https://github.com/Timo274/svx
cd svx
cargo install --path crates/svx-cli
svx --help
```

## Quickstart

On the destination server:

```bash
svx receive --out /home/sol/validator-keypair.json
# prints a transfer code like `371-extra-bliss` and an expected SAS
```

On the source server:

```bash
svx send \
    --identity /home/sol/validator-keypair.json \
    --code 371-extra-bliss \
    --label mainnet-validator
```

The CLIs will ask each operator to confirm the 4-word SAS matches
out-of-band. After confirmation, the encrypted envelope flows over the
relay and the destination writes `validator-keypair.json` with mode
`0600`.

Print the full zero-downtime hot-swap procedure offline:

```bash
svx hot-swap
```

## Self-host the relay

Everything you need to run your own relay is in `crates/svx-relay`:

```bash
cargo run --release -p svx-relay
# or, with Docker:
docker build -t svx-relay -f crates/svx-relay/Dockerfile .
docker run --rm -p 8080:8080 svx-relay
```

Point clients at it with `SVX_RELAY=wss://relay.example.com svx send ...`.

## Repository layout

```
crates/
  svx-core/    # encryption, identity parsing, SAS, protocol types (library, pure)
  svx-relay/   # rendezvous WebSocket server (axum)
  svx-cli/     # `svx` binary: receive / send / hot-swap / inspect
web/           # marketing + docs site (Vite + React + Tailwind)
```

## License

Apache-2.0. See [LICENSE](./LICENSE).

Built for the [Superteam Ukraine validator identity transfer bounty](https://earn.superteam.fun/).
