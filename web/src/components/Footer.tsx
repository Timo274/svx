export function Footer() {
  return (
    <footer className="mt-24 border-t border-ink-700">
      <div className="mx-auto w-full max-w-5xl px-6 py-8 flex items-center justify-between text-sm text-slate-400 flex-wrap gap-3">
        <div>
          <span className="text-slate-300 font-medium">svx</span>
          <span className="mx-2">·</span>
          <span>Apache-2.0</span>
          <span className="mx-2">·</span>
          <a href="https://github.com/Timo274/svx" className="hover:text-white">GitHub</a>
        </div>
        <div>Built for Superteam Ukraine · not affiliated with Solana Foundation.</div>
      </div>
    </footer>
  );
}
