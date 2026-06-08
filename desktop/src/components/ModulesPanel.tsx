import { useCallback, useEffect, useMemo, useState } from "react";
import type { Notify } from "../App";
import { api } from "../api";
import type { KernelModule, ModuleInfo } from "../types";

interface Props {
  notify: Notify;
}

export default function ModulesPanel({ notify }: Props) {
  const [mods, setMods] = useState<KernelModule[]>([]);
  const [search, setSearch] = useState("");
  const [expanded, setExpanded] = useState<string | null>(null);
  const [info, setInfo] = useState<ModuleInfo | null>(null);
  const [loadName, setLoadName] = useState("");
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      setMods(await api.kernelModules());
    } catch (e) {
      notify(String(e), "err");
    }
  }, [notify]);

  useEffect(() => {
    load();
  }, [load]);

  const visible = useMemo(() => {
    const q = search.trim().toLowerCase();
    return q ? mods.filter((m) => m.name.toLowerCase().includes(q)) : mods;
  }, [mods, search]);

  const expand = async (name: string) => {
    if (expanded === name) {
      setExpanded(null);
      return;
    }
    setExpanded(name);
    setInfo(null);
    try {
      setInfo(await api.kernelModuleInfo(name));
    } catch (e) {
      notify(String(e), "err");
    }
  };

  const unload = async (m: KernelModule) => {
    if (m.refcount > 0 || m.used_by.length > 0) {
      if (!confirm(`${m.name} is in use (${m.refcount}). Unload anyway?`)) return;
    }
    setBusy(true);
    try {
      await api.kernelModuleUnload(m.name);
      notify(`Unloaded ${m.name}`, "ok");
      await load();
    } catch (e) {
      notify(String(e), "err");
    } finally {
      setBusy(false);
    }
  };

  const loadModule = async () => {
    const name = loadName.trim();
    if (!name) return;
    setBusy(true);
    try {
      await api.kernelModuleLoad(name);
      notify(`Loaded ${name}`, "ok");
      setLoadName("");
      await load();
    } catch (e) {
      notify(String(e), "err");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div>
      <div className="row between">
        <div>
          <div className="panel-title">🧩 Kernel modules</div>
          <div className="panel-sub">
            {mods.length} loaded · load/unload needs root (pkexec).
          </div>
        </div>
        <div className="row" style={{ gap: 8 }}>
          <input
            className="prop-input"
            style={{ width: 160 }}
            placeholder="modprobe name…"
            value={loadName}
            disabled={busy}
            onChange={(e) => setLoadName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && loadModule()}
          />
          <button className="btn primary" disabled={busy || !loadName.trim()} onClick={loadModule}>
            ＋ Load
          </button>
          <button className="iconbtn" onClick={load} title="Refresh">
            ⟳
          </button>
        </div>
      </div>

      <input
        className="search"
        style={{ width: "100%", margin: "8px 0 14px" }}
        placeholder="Search modules…"
        value={search}
        onChange={(e) => setSearch(e.target.value)}
      />

      {visible.map((m) => (
        <div key={m.name} className="card" style={{ padding: 0 }}>
          <button
            className="row between"
            style={{
              width: "100%",
              background: "none",
              padding: "12px 16px",
              textAlign: "left",
            }}
            onClick={() => expand(m.name)}
          >
            <div>
              <span style={{ fontWeight: 600, fontFamily: "ui-monospace, monospace" }}>
                {m.name}
              </span>
              <span className="panel-sub" style={{ margin: "0 0 0 10px" }}>
                {m.size_kb} KB · refs {m.refcount}
                {m.used_by.length ? ` · used by ${m.used_by.join(", ")}` : ""}
              </span>
            </div>
            <span style={{ color: "var(--overlay)" }}>{expanded === m.name ? "▾" : "▸"}</span>
          </button>

          {expanded === m.name && (
            <div style={{ padding: "0 16px 14px" }}>
              {info === null ? (
                <div className="panel-sub">Reading modinfo…</div>
              ) : (
                <>
                  <table className="prop-table">
                    <tbody>
                      {info.description && <Row k="Description" v={info.description} />}
                      {info.author && <Row k="Author" v={info.author} />}
                      {info.license && <Row k="License" v={info.license} />}
                      {info.version && <Row k="Version" v={info.version} />}
                      {info.depends.length > 0 && (
                        <Row k="Depends" v={info.depends.join(", ")} />
                      )}
                      {info.filename && <Row k="File" v={info.filename} />}
                    </tbody>
                  </table>
                  <div className="row" style={{ marginTop: 12 }}>
                    <button className="btn danger" disabled={busy} onClick={() => unload(m)}>
                      ⏏ Unload
                    </button>
                  </div>
                </>
              )}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

function Row({ k, v }: { k: string; v: string }) {
  return (
    <tr>
      <td className="k">{k}</td>
      <td className="v">{v}</td>
    </tr>
  );
}
