import { useCallback, useEffect, useMemo, useState } from "react";
import type { Notify } from "../App";
import { api } from "../api";
import type { Service } from "../types";

const DOT: Record<string, string> = {
  active: "🟢",
  inactive: "⚪",
  failed: "🔴",
};

interface Props {
  notify: Notify;
  confirmDestructive: boolean;
}

export default function ServicesPanel({ notify, confirmDestructive }: Props) {
  const [services, setServices] = useState<Service[]>([]);
  const [q, setQ] = useState("");
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      setServices(await api.servicesList());
    } catch (e) {
      notify(String(e), "err");
    }
  }, [notify]);

  useEffect(() => {
    load();
    const t = window.setInterval(load, 6000);
    return () => window.clearInterval(t);
  }, [load]);

  const act = async (name: string, action: string) => {
    setBusy(true);
    try {
      await api.serviceAction(name, action);
      notify(`${action} → ${name}`, "ok");
      await load();
    } catch (e) {
      notify(String(e), "err");
    } finally {
      setBusy(false);
    }
  };

  const shown = useMemo(() => {
    const ql = q.trim().toLowerCase();
    const list = ql
      ? services.filter(
          (s) => s.name.toLowerCase().includes(ql) || s.description.toLowerCase().includes(ql)
        )
      : services;
    return [...list].sort(
      (a, b) =>
        Number(b.active === "active") - Number(a.active === "active") ||
        a.name.localeCompare(b.name)
    );
  }, [services, q]);

  return (
    <div>
      <div className="row between">
        <div>
          <div className="panel-title">🛠 Services</div>
          <div className="panel-sub">
            systemd services. Start/stop/restart prompts for authorization.
          </div>
        </div>
        <button className="iconbtn" onClick={load} title="Refresh">
          ⟳
        </button>
      </div>

      <input
        className="search"
        style={{ width: "100%", margin: "0 0 8px" }}
        placeholder="Filter services…"
        value={q}
        onChange={(e) => setQ(e.target.value)}
      />

      {shown.length === 0 && <div className="empty">No services match.</div>}

      {shown.map((s) => {
        const running = s.active === "active";
        return (
          <div key={s.name} className={`media-item ${running ? "active" : ""}`}>
            <span className="ico" style={{ fontSize: 16 }}>
              {DOT[s.active] ?? "⚫"}
            </span>
            <div className="meta">
              <div className="name">{s.name}</div>
              <div className="desc">
                {s.active} · {s.sub}
                {s.description ? ` · ${s.description}` : ""}
              </div>
            </div>
            {running ? (
              <>
                <button className="btn ghost" disabled={busy} onClick={() => act(s.name, "restart")}>
                  Restart
                </button>
                <button
                  className="btn danger"
                  disabled={busy}
                  onClick={() => {
                    if (confirmDestructive && !confirm(`Stop ${s.name}?`)) return;
                    act(s.name, "stop");
                  }}
                >
                  Stop
                </button>
              </>
            ) : (
              <button className="btn primary" disabled={busy} onClick={() => act(s.name, "start")}>
                Start
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}
