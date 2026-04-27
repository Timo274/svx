import { useEffect, useState } from "react";

type Health = { ok: boolean; active_rooms: number; protocol_version: number };

const PUBLIC_RELAY = (import.meta as unknown as { env: Record<string, string> }).env
  ?.VITE_RELAY_URL || "https://svx-relay.fly.dev";

export function RelayStatus() {
  const [state, setState] = useState<
    { kind: "loading" } | { kind: "ok"; data: Health } | { kind: "err"; msg: string }
  >({ kind: "loading" });

  useEffect(() => {
    let cancelled = false;
    const url = PUBLIC_RELAY.replace(/\/$/, "") + "/health";
    fetch(url, { cache: "no-store" })
      .then(async (r) => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return r.json() as Promise<Health>;
      })
      .then((data) => !cancelled && setState({ kind: "ok", data }))
      .catch((e) => !cancelled && setState({ kind: "err", msg: String(e) }));
    return () => { cancelled = true; };
  }, []);

  return (
    <section className="card">
      <div className="flex items-center justify-between gap-4 flex-wrap">
        <div>
          <p className="text-xs uppercase tracking-widest text-accent-cyan">public relay</p>
          <h3 className="mt-1 font-semibold">{PUBLIC_RELAY}</h3>
          <p className="mt-1 text-sm text-slate-400">
            Operated as a best-effort community service. Self-host in one binary if you want
            zero third-party touch.
          </p>
        </div>
        <StatusBadge state={state} />
      </div>
    </section>
  );
}

function StatusBadge({
  state,
}: { state: { kind: "loading" } | { kind: "ok"; data: Health } | { kind: "err"; msg: string } }) {
  if (state.kind === "loading") {
    return <span className="chip">checking…</span>;
  }
  if (state.kind === "err") {
    return (
      <span className="chip border-red-700 text-red-300">
        <Dot className="bg-red-400" /> unreachable · {state.msg}
      </span>
    );
  }
  return (
    <span className="chip border-emerald-700 text-emerald-300">
      <Dot className="bg-emerald-400" /> online · {state.data.active_rooms} active room{state.data.active_rooms === 1 ? "" : "s"} · protocol v{state.data.protocol_version}
    </span>
  );
}

function Dot({ className }: { className: string }) {
  return <span className={`h-2 w-2 rounded-full inline-block ${className}`} />;
}
