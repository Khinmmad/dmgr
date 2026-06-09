import { useCallback, useEffect, useState } from "react";
import type { Notify } from "../App";
import { api } from "../api";
import type { BtState } from "../types";

function iconFor(icon: string): string {
  if (icon.includes("audio")) return "🎧";
  if (icon.includes("input-keyboard")) return "⌨";
  if (icon.includes("input-mouse")) return "🖱";
  if (icon.includes("phone")) return "📱";
  return "🔵";
}

interface Props {
  notify: Notify;
  os: string;
}

export default function BluetoothPanel({ notify, os }: Props) {
  const isWindows = os === "windows";
  const [state, setState] = useState<BtState | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setState(await api.btState());
    } catch (e) {
      notify(String(e), "err");
    }
  }, [notify]);

  useEffect(() => {
    load();
    const t = window.setInterval(load, 5000);
    return () => window.clearInterval(t);
  }, [load]);

  const act = async (key: string, fn: () => Promise<void>, msg: string) => {
    setBusy(key);
    try {
      await fn();
      notify(msg, "ok");
      await load();
    } catch (e) {
      notify(String(e), "err");
    } finally {
      setBusy(null);
    }
  };

  if (state && !state.available) {
    return (
      <div>
        <div className="panel-title">🔵 Bluetooth</div>
        <div className="empty">
          {isWindows ? (
            <>No Bluetooth adapter found.</>
          ) : (
            <>
              <code>bluetoothctl</code> not found. Install <code>bluez-utils</code> to manage
              Bluetooth devices.
            </>
          )}
        </div>
      </div>
    );
  }

  const devices = state?.devices ?? [];
  const sorted = [...devices].sort(
    (a, b) => Number(b.connected) - Number(a.connected) || a.name.localeCompare(b.name)
  );

  return (
    <div>
      <div className="row between">
        <div>
          <div className="panel-title">🔵 Bluetooth</div>
          <div className="panel-sub">
            {isWindows
              ? "Paired devices and adapter power. Pair or connect from Windows Settings."
              : "Connect and manage paired Bluetooth devices."}
          </div>
        </div>
        <div className="row" style={{ gap: 10 }}>
          <span className="panel-sub" style={{ margin: 0 }}>
            Adapter {state?.powered ? "on" : "off"}
          </span>
          <button
            className={`switch ${state?.powered ? "on" : ""}`}
            disabled={busy === "power"}
            onClick={() =>
              act(
                "power",
                () => api.btSetPower(!state?.powered),
                state?.powered ? "Adapter off" : "Adapter on"
              )
            }
          />
          {isWindows && (
            <button
              className="iconbtn"
              onClick={() => api.openBluetoothSettings().catch((e) => notify(String(e), "err"))}
              title="Open Windows Bluetooth settings (pair / connect)"
            >
              ⚙ Settings
            </button>
          )}
          <button className="iconbtn" onClick={load} title="Refresh">
            ⟳
          </button>
        </div>
      </div>

      <div className="section-h">Paired devices</div>
      {sorted.length === 0 && <div className="empty">No paired devices.</div>}
      {sorted.map((d) => (
        <div key={d.mac} className={`media-item ${d.connected ? "active" : ""}`}>
          <span className="ico">{iconFor(d.icon)}</span>
          <div className="meta">
            <div className="name">{d.name}</div>
            <div className="desc">
              {d.mac} · {d.connected ? "connected" : d.paired ? "paired" : "—"}
              {d.trusted ? " · trusted" : ""}
            </div>
          </div>

          {isWindows ? (
            // Windows: read-only list (connect/disconnect is managed by the OS).
            <span className={d.connected ? "badge-active" : "panel-sub"} style={{ margin: 0 }}>
              {d.connected ? "● Connected" : "Paired"}
            </span>
          ) : (
            <>
              <button
                className="btn ghost"
                disabled={busy === d.mac}
                onClick={() =>
                  act(
                    d.mac,
                    () => api.btSetTrust(d.mac, !d.trusted),
                    d.trusted ? "Untrusted" : "Trusted"
                  )
                }
              >
                {d.trusted ? "★" : "☆"}
              </button>

              {d.connected ? (
                <button
                  className="btn danger"
                  disabled={busy === d.mac}
                  onClick={() =>
                    act(d.mac, () => api.btDisconnect(d.mac), `Disconnected ${d.name}`)
                  }
                >
                  Disconnect
                </button>
              ) : (
                <button
                  className="btn primary"
                  disabled={busy === d.mac}
                  onClick={() => act(d.mac, () => api.btConnect(d.mac), `Connected ${d.name}`)}
                >
                  Connect
                </button>
              )}
            </>
          )}
        </div>
      ))}
    </div>
  );
}
