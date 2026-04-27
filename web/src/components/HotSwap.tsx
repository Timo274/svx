export function HotSwap() {
  return (
    <section id="hot-swap" className="space-y-6">
      <div>
        <p className="text-xs uppercase tracking-widest text-accent-cyan">zero-downtime transition</p>
        <h2 className="mt-2 text-2xl md:text-3xl font-semibold tracking-tight">
          svx fits inside the canonical identity hot-swap flow.
        </h2>
        <p className="mt-2 text-slate-300 max-w-3xl">
          Moving a staked validator identity is a careful dance: never run two nodes with the
          same identity at once, never miss votes if you can help it. svx ships the key — the
          rest is <code>solana-validator set-identity</code>.
        </p>
      </div>

      <ol className="card space-y-4 list-decimal list-inside text-slate-200">
        <li>
          On <b>node A</b> (currently voting): set an <i>unstaked</i> identity so A stops
          voting as the real key.
          <pre className="code-block mt-2">solana-validator -l /mnt/ledger set-identity /home/sol/unfunded-A.json</pre>
        </li>
        <li>
          Move the real identity keypair from A to B using svx:
          <pre className="code-block mt-2">{`# on node B
svx receive --out /home/sol/validator-keypair.json
# on node A
svx send --identity /home/sol/validator-keypair.json --code <CODE>`}</pre>
        </li>
        <li>
          On <b>node B</b>: set the real identity live, <b>without restarting</b>.
          <pre className="code-block mt-2">{`solana-validator -l /mnt/ledger set-identity \\
  --require-tower /home/sol/validator-keypair.json`}</pre>
        </li>
        <li>
          Verify on B with <code>solana-validator monitor</code> that votes are landing.
        </li>
        <li>
          On <b>node A</b>: shred the now-duplicate keypair file.
          <pre className="code-block mt-2">shred -u /home/sol/validator-keypair.json</pre>
        </li>
      </ol>
      <p className="text-xs text-slate-500">
        The exact same text is also available offline via <code>svx hot-swap</code>.
      </p>
    </section>
  );
}
