import { LockIcon, ShieldIcon, EyeOffIcon, CheckIcon } from "./icons";

export function Security() {
  return (
    <section id="security" className="space-y-8">
      <div>
        <p className="text-xs uppercase tracking-widest text-accent-cyan">security model</p>
        <h2 className="mt-2 text-2xl md:text-3xl font-semibold tracking-tight">
          The relay is untrusted by design.
        </h2>
        <p className="mt-2 text-slate-300 max-w-3xl">
          Every guarantee svx gives you — confidentiality, integrity, and authenticity of the
          transferred identity — is enforced in the CLI on both ends. The rendezvous server
          is only a postman for opaque ciphertext.
        </p>
      </div>

      <div className="grid md:grid-cols-2 gap-4">
        <Tile
          icon={<LockIcon className="h-5 w-5 text-accent-teal" />}
          title="End-to-end encryption via age"
          body="Receiver generates an ephemeral X25519 keypair locally. Sender encrypts using the age v1 format (X25519 + ChaCha20-Poly1305). The private key never leaves the receiver."
        />
        <Tile
          icon={<ShieldIcon className="h-5 w-5 text-accent-purple" />}
          title="Short Authentication String (SAS)"
          body="Both sides derive a 4-word SAS from the recipient pubkey and transfer code. Operators compare it out-of-band (voice, Signal, DM). Mismatched SAS = aborted transfer."
        />
        <Tile
          icon={<CheckIcon className="h-5 w-5 text-accent-cyan" />}
          title="Identity-signed attestation"
          body="The sender signs the envelope with the Ed25519 validator identity itself. The receiver verifies that the key it just decrypted really corresponds to the declared pubkey — no substitution possible mid-flight."
        />
        <Tile
          icon={<EyeOffIcon className="h-5 w-5 text-accent-teal" />}
          title="Zero-trust relay"
          body="The public relay is stateless, ciphertext-only, with a 10-minute idle TTL and a 30-minute hard lifetime. Self-host your own in one binary if you want zero third-party touch."
        />
      </div>

      <div className="card bg-ink-800/40">
        <h3 className="font-semibold">Threats we explicitly defend against</h3>
        <ul className="mt-3 grid md:grid-cols-2 gap-y-2 text-sm text-slate-300 list-disc list-inside">
          <li>Passive eavesdropper on the internet path or relay</li>
          <li>A malicious or compromised relay substituting keys</li>
          <li>Replay of a previously recorded transfer</li>
          <li>Accidental overwrite of the destination identity file</li>
          <li>Corrupted or truncated identity JSON</li>
          <li>Cross-session confusion (wrong code, wrong recipient)</li>
        </ul>
      </div>
    </section>
  );
}

function Tile({
  icon, title, body,
}: { icon: React.ReactNode; title: string; body: string }) {
  return (
    <div className="card">
      <div className="flex items-center gap-2">
        {icon}
        <h3 className="font-semibold">{title}</h3>
      </div>
      <p className="mt-2 text-sm text-slate-400">{body}</p>
    </div>
  );
}
