export function Steps() {
  return (
    <section id="steps" className="space-y-8">
      <SectionHeader
        eyebrow="how it works"
        title="Three commands. No ceremony."
        subtitle="One tool, two servers. Identity never leaves either host in plaintext."
      />

      <div className="grid md:grid-cols-3 gap-4">
        <StepCard
          n={1}
          title="On the destination: receive"
          command={`# node B (target server)
svx receive \\
  --out /home/sol/validator-keypair.json`}
          desc="Generates a one-time transfer code and prints the expected SAS. Waits for the sender to connect."
        />
        <StepCard
          n={2}
          title="On the source: send"
          command={`# node A (current holder)
svx send \\
  --identity /home/sol/validator-keypair.json \\
  --code 371-extra-bliss`}
          desc="Encrypts the identity to the receiver's ephemeral public key and ships the ciphertext via the relay."
        />
        <StepCard
          n={3}
          title="Hot-swap the live voter"
          command={`# node B, after receive completes
solana-validator -l /mnt/ledger \\
  set-identity --require-tower \\
  /home/sol/validator-keypair.json`}
          desc="Swap the active identity without restarting. svx prints the full zero-downtime checklist via svx hot-swap."
        />
      </div>

      <Install />
    </section>
  );
}

function SectionHeader({
  eyebrow, title, subtitle,
}: { eyebrow: string; title: string; subtitle?: string }) {
  return (
    <div>
      <p className="text-xs uppercase tracking-widest text-accent-cyan">{eyebrow}</p>
      <h2 className="mt-2 text-2xl md:text-3xl font-semibold tracking-tight">{title}</h2>
      {subtitle && <p className="mt-2 text-slate-300">{subtitle}</p>}
    </div>
  );
}

function StepCard({
  n, title, command, desc,
}: { n: number; title: string; command: string; desc: string }) {
  return (
    <div className="card flex flex-col gap-3">
      <div className="flex items-center gap-2">
        <span className="h-6 w-6 rounded-full bg-gradient-to-br from-accent-purple to-accent-teal text-ink-900 text-xs font-bold flex items-center justify-center">
          {n}
        </span>
        <h3 className="font-semibold">{title}</h3>
      </div>
      <pre className="code-block whitespace-pre-wrap">{command}</pre>
      <p className="text-sm text-slate-400">{desc}</p>
    </div>
  );
}

function Install() {
  return (
    <div id="install" className="card">
      <h3 className="font-semibold text-lg">Install</h3>
      <p className="mt-1 text-sm text-slate-400">
        Build from source today; prebuilt binaries are coming with the first tagged release.
      </p>
      <pre className="code-block mt-4 whitespace-pre-wrap">{`# requires Rust 1.85+
git clone https://github.com/Timo274/svx
cd svx
cargo install --path crates/svx-cli

svx --help`}</pre>
    </div>
  );
}
