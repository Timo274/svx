import { GithubIcon, DownloadIcon, LockIcon } from "./icons";

const REPO = "https://github.com/Timo274/svx";

export function Hero() {
  return (
    <section className="pt-6">
      <header className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <LogoMark />
          <span className="font-semibold tracking-tight text-lg">svx</span>
        </div>
        <nav className="flex items-center gap-2 text-sm">
          <a className="btn-ghost" href="#steps">How it works</a>
          <a className="btn-ghost" href="#security">Security</a>
          <a className="btn-ghost" href={REPO} target="_blank" rel="noreferrer">
            <GithubIcon className="h-4 w-4" /> GitHub
          </a>
        </nav>
      </header>

      <div className="mt-14 md:mt-20 grid md:grid-cols-[1.3fr_1fr] gap-10 items-center">
        <div>
          <span className="chip">
            <LockIcon className="h-3.5 w-3.5 text-accent-teal" />
            end-to-end encrypted · open source · Rust
          </span>
          <h1 className="mt-5 text-4xl md:text-5xl font-semibold leading-tight tracking-tight">
            Move your Solana validator identity
            <span className="block bg-gradient-to-r from-accent-purple via-accent-cyan to-accent-teal bg-clip-text text-transparent">
              between servers — securely.
            </span>
          </h1>
          <p className="mt-5 text-slate-300 text-lg max-w-2xl">
            <code className="text-accent-teal">svx</code> is a tiny, audited-by-design CLI that
            hot-swaps a validator's <code>validator-keypair.json</code> from one machine to
            another without the private key ever leaving the two endpoints in plaintext.
          </p>
          <div className="mt-7 flex flex-wrap gap-3">
            <a href="#install" className="btn-primary">
              <DownloadIcon className="h-4 w-4" /> Install svx
            </a>
            <a href={REPO} target="_blank" rel="noreferrer" className="btn-ghost">
              <GithubIcon className="h-4 w-4" /> View on GitHub
            </a>
          </div>
          <ul className="mt-8 grid gap-2 text-sm text-slate-300">
            <li>· X25519 + ChaCha20-Poly1305 via the <code>age</code> format</li>
            <li>· 4-word Short Authentication String (SAS) protects against MITM</li>
            <li>· Identity-signed attestation so the receiver verifies it's the real key</li>
            <li>· Relay never sees plaintext — run your own or use the public one</li>
          </ul>
        </div>

        <TerminalPreview />
      </div>
    </section>
  );
}

function LogoMark() {
  return (
    <div className="relative h-8 w-8">
      <div className="absolute inset-0 rounded-lg bg-gradient-to-br from-accent-purple via-accent-cyan to-accent-teal blur-[4px] opacity-70" />
      <div className="absolute inset-[2px] rounded-md bg-ink-900 flex items-center justify-center">
        <span className="text-sm font-bold bg-gradient-to-r from-accent-purple to-accent-teal bg-clip-text text-transparent">sv</span>
      </div>
    </div>
  );
}

function TerminalPreview() {
  return (
    <div className="card shadow-glow !p-0 overflow-hidden">
      <div className="flex items-center gap-1.5 px-4 py-2 border-b border-ink-500 bg-ink-800">
        <span className="h-2.5 w-2.5 rounded-full bg-[#ff5f56]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#ffbd2e]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#27c93f]" />
        <span className="ml-3 text-xs text-slate-400">dst ~ sol@node-b</span>
      </div>
      <pre className="p-5 text-[12.5px] leading-relaxed text-slate-200 whitespace-pre-wrap">
{`$ svx receive --out /home/sol/validator-keypair.json
svx  secure validator identity transfer

▸ receive: wait for an inbound validator identity
  generated ephemeral recipient: age1mry…nx9rszw53wr

▸ transfer code: `}<span className="text-accent-cyan font-semibold">371-extra-bliss</span>{`
  expected SAS: `}<span className="text-accent-teal font-semibold">hunt-brisk-imp-cycle</span>{`
  waiting for sender to join…
  sender joined. awaiting encrypted payload…

▸ transfer complete
  wrote identity to /home/sol/validator-keypair.json
  pubkey: GRE3XGnbtWDiwyCMuDCSCXLMSRWwHA5Q6R8m4NcrksCa
`}
      </pre>
    </div>
  );
}
