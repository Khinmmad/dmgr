import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { Notify } from "../App";
import { api } from "../api";
import { useAliases } from "../aliases";
import { useFavorites } from "../favorites";
import type { BtDevice, BtState } from "../types";

function iconFor(icon: string): string {
  if (icon.includes("audio")) return "🎧";
  if (icon.includes("input-keyboard")) return "⌨";
  if (icon.includes("input-mouse")) return "🖱";
  if (icon.includes("phone")) return "📱";
  return "🔵";
}

function typeLabel(icon: string): string {
  if (icon.includes("audio")) return "Audio";
  if (icon.includes("input-keyboard")) return "Keyboard";
  if (icon.includes("input-mouse")) return "Mouse";
  if (icon.includes("input-gaming")) return "Controller";
  if (icon.includes("phone")) return "Phone";
  if (icon.includes("computer")) return "Computer";
  return "Device";
}

interface Props {
  notify: Notify;
  os: string;
  notifications: boolean;
}

export default function BluetoothPanel({ notify, os, notifications }: Props) {
  const isWindows = os === "windows";
  const [state, setState] = useState<BtState | null>(null);
  // Per-action in-flight guard. Prevents overlapping bluetoothctl calls when
  // the user double-clicks or rapidly toggles. Keys: "power", "scan",
  // "trust:<mac>", "connect:<mac>", "disconnect:<mac>", "remove:<mac>",
  // "pair:<mac>".
  const [inFlight, setInFlight] = useState<Set<string>>(new Set());
  // True while a scan is running, for the spinner.
  const [scanning, setScanning] = useState(false);
  // mac of the device whose details modal is open (null = closed).
  const [detailMac, setDetailMac] = useState<string | null>(null);
  const { map: aliases, name: aliasName, rename } = useAliases();
  const { has: isFav, toggle: toggleFav } = useFavorites();
  const [q, setQ] = useState("");
  // Previous connected MACs, to detect connect/disconnect transitions.
  const prevConn = useRef<Set<string> | null>(null);

  const load = useCallback(async () => {
    try {
      const s = await api.btState();
      const conn = new Set(s.devices.filter((d) => d.connected).map((d) => d.mac));
      // Skip the very first load (prevConn null) so we don't announce the
      // already-connected devices on open.
      if (prevConn.current && notifications) {
        for (const d of s.devices) {
          const was = prevConn.current.has(d.mac);
          if (d.connected && !was) notify(`${d.name} connected`, "ok");
          else if (!d.connected && was) notify(`${d.name} disconnected`, "ok");
        }
      }
      prevConn.current = conn;
      setState(s);
    } catch (e) {
      notify(String(e), "err");
    }
  }, [notify, notifications]);

  useEffect(() => {
    load();
    // Event-driven: the backend's bluetoothctl monitor emits "bluetooth-changed"
    // on any BlueZ change; debounce bursts (e.g. RSSI churn during a scan).
    let t: number | undefined;
    const unlisten = listen("bluetooth-changed", () => {
      window.clearTimeout(t);
      t = window.setTimeout(load, 500);
    });
    // Fallback poll in case an event is missed — slow on Linux (events do the
    // work), unchanged on Windows (no BlueZ event stream there).
    const iv = window.setInterval(load, isWindows ? 5000 : 20000);
    return () => {
      unlisten.then((fn) => fn());
      window.clearTimeout(t);
      window.clearInterval(iv);
    };
  }, [load, isWindows]);

  const setBusy = (key: string, on: boolean) =>
    setInFlight((prev) => {
      const next = new Set(prev);
      if (on) next.add(key);
      else next.delete(key);
      return next;
    });

  const isBusy = (key: string) => inFlight.has(key);

  /**
   * Run an action, guarding against overlapping calls (key-based), surfacing
   * errors via the toast and refreshing state afterwards.
   */
  const act = async (key: string, fn: () => Promise<void>, msg: string) => {
    if (isBusy(key)) return;
    setBusy(key, true);
    try {
      await fn();
      notify(msg, "ok");
      await load();
    } catch (e) {
      notify(String(e), "err");
    } finally {
      setBusy(key, false);
    }
  };

  const onScan = async () => {
    if (scanning || isBusy("scan")) return;
    setScanning(true);
    setBusy("scan", true);
    try {
      // 10 s is a good default: long enough to catch most devices, short
      // enough that the radio isn't pegged.
      await api.btScan(10);
      notify("Scan complete", "ok");
      await load();
    } catch (e) {
      notify(String(e), "err");
    } finally {
      setScanning(false);
      setBusy("scan", false);
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

  // If the daemon is up but the controller isn't, surface that distinctly.
  const daemonDown = state?.available && !state.powered && state.devices.length === 0;

  const devices = state?.devices ?? [];
  const ql = q.trim().toLowerCase();
  const dn = (d: BtDevice) => aliasName(`bt:${d.mac}`, d.name);
  const matches = (d: BtDevice) =>
    !ql || dn(d).toLowerCase().includes(ql) || d.mac.toLowerCase().includes(ql);
  // Favorites first, then connected, then by (alias) name.
  const byPreference = (a: BtDevice, b: BtDevice) =>
    Number(isFav(`bt:${b.mac}`)) - Number(isFav(`bt:${a.mac}`)) ||
    Number(b.connected) - Number(a.connected) ||
    dn(a).localeCompare(dn(b));
  const paired = devices.filter((d) => d.paired && matches(d)).sort(byPreference);
  // Discovered = seen during a scan but not yet paired (Linux only; on Windows
  // every listed device is already paired and discovery is the OS's job).
  const discovered = devices.filter((d) => !d.paired && matches(d)).sort(byPreference);
  // Looked up live from state so the modal reflects refreshes (battery, trust…).
  const detail = detailMac ? devices.find((d) => d.mac === detailMac) ?? null : null;

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
            disabled={isBusy("power")}
            onClick={() =>
              act(
                "power",
                () => api.btSetPower(!state?.powered),
                state?.powered ? "Adapter off" : "Adapter on"
              )
            }
          />
          {!isWindows && (
            <button
              className="iconbtn"
              disabled={scanning}
              onClick={onScan}
              title="Discover nearby devices for 10s"
            >
              {scanning ? "Scanning…" : "Scan"}
            </button>
          )}
          {isWindows && (
            <button
              className="iconbtn"
              onClick={() => api.openBluetoothSettings().catch((e) => notify(String(e), "err"))}
              title="Open Windows Bluetooth settings (pair / connect)"
            >
              ⚙ Settings
            </button>
          )}
          <button className="iconbtn" onClick={load} title="Refresh" disabled={scanning}>
            ⟳
          </button>
        </div>
      </div>

      {!isWindows && (
        <input
          className="search"
          style={{ width: "100%", marginBottom: 4 }}
          placeholder="Filter devices…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
      )}

      {daemonDown && (
        <div className="card" style={{ color: "var(--subtext)" }}>
          Bluetooth adapter is not responding. Make sure <code>bluetoothd</code> is running:
          <br />
          <code>sudo systemctl enable --now bluetooth</code>
        </div>
      )}

      <div className="section-h">Paired devices</div>
      {paired.length === 0 && !daemonDown && (
        <div className="empty">No paired devices. Click "Scan" to discover nearby ones.</div>
      )}
      {paired.map((d) => (
        <div key={d.mac} className={`media-item ${d.connected ? "active" : ""}`}>
          <span className="ico">{iconFor(d.icon)}</span>
          <div className="meta">
            <div className="name">{aliasName(`bt:${d.mac}`, d.name)}</div>
            <div className="desc">
              {d.mac} · {d.connected ? "connected" : d.paired ? "paired" : "—"}
              {d.trusted ? " · auto-connect" : ""}
              {d.battery != null ? ` · 🔋 ${d.battery}%` : ""}
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
                style={{ opacity: isFav(`bt:${d.mac}`) ? 1 : 0.35, padding: "8px 10px" }}
                onClick={() => toggleFav(`bt:${d.mac}`)}
                title={isFav(`bt:${d.mac}`) ? "Unpin from top" : "Pin to top"}
              >
                ⭐
              </button>
              <button
                className="btn ghost"
                onClick={() => setDetailMac(d.mac)}
                title="Device details (battery, signal, auto-connect)"
              >
                Details
              </button>

              {d.connected ? (
                <button
                  className="btn danger"
                  disabled={isBusy(`disconnect:${d.mac}`)}
                  onClick={() =>
                    act(
                      `disconnect:${d.mac}`,
                      () => api.btDisconnect(d.mac),
                      `Disconnected ${d.name}`
                    )
                  }
                >
                  Disconnect
                </button>
              ) : (
                <button
                  className="btn primary"
                  disabled={isBusy(`connect:${d.mac}`)}
                  onClick={() =>
                    act(
                      `connect:${d.mac}`,
                      () => api.btConnect(d.mac),
                      `Connected ${d.name}`
                    )
                  }
                >
                  Connect
                </button>
              )}

              <button
                className="btn ghost"
                disabled={isBusy(`remove:${d.mac}`)}
                onClick={() =>
                  act(`remove:${d.mac}`, () => api.btRemove(d.mac), `Removed ${d.name}`)
                }
                title="Unpair (forget) this device"
              >
                Unpair
              </button>
            </>
          )}
        </div>
      ))}

      {!isWindows && discovered.length > 0 && (
        <>
          <div className="section-h">Discovered devices</div>
          {discovered.map((d) => (
            <div key={d.mac} className="media-item">
              <span className="ico">{iconFor(d.icon)}</span>
              <div className="meta">
                <div className="name">{aliasName(`bt:${d.mac}`, d.name)}</div>
                <div className="desc">{d.mac} · not paired</div>
              </div>
              <button
                className="btn primary"
                disabled={isBusy(`pair:${d.mac}`)}
                onClick={() =>
                  act(`pair:${d.mac}`, () => api.btPair(d.mac), `Paired ${d.name}`)
                }
                title="Pair (bond) with this device"
              >
                {isBusy(`pair:${d.mac}`) ? "Pairing…" : "Pair"}
              </button>
            </div>
          ))}
        </>
      )}

      {detail && (
        <div className="modal-backdrop" onClick={() => setDetailMac(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="row between">
              <div className="row" style={{ gap: 12 }}>
                <span style={{ fontSize: 30 }}>{iconFor(detail.icon)}</span>
                <div className="panel-title" style={{ margin: 0 }}>
                  {aliasName(`bt:${detail.mac}`, detail.name)}
                </div>
              </div>
              <button className="iconbtn" onClick={() => setDetailMac(null)} title="Close">
                ✕
              </button>
            </div>

            <table className="prop-table" style={{ marginTop: 14 }}>
              <tbody>
                <tr>
                  <td className="k">Address</td>
                  <td className="v">{detail.mac}</td>
                </tr>
                <tr>
                  <td className="k">Type</td>
                  <td className="v">{typeLabel(detail.icon)}</td>
                </tr>
                <tr>
                  <td className="k">Status</td>
                  <td className="v">
                    {detail.connected ? "Connected" : detail.paired ? "Paired" : "Discovered"}
                  </td>
                </tr>
                {detail.battery != null && (
                  <tr>
                    <td className="k">Battery</td>
                    <td className="v">🔋 {detail.battery}%</td>
                  </tr>
                )}
                {detail.rssi != null && (
                  <tr>
                    <td className="k">Signal</td>
                    <td className="v">{detail.rssi} dBm</td>
                  </tr>
                )}
              </tbody>
            </table>

            <div className="set-row">
              <div>
                <div className="set-label">Custom name</div>
                <div className="panel-sub" style={{ margin: 0 }}>
                  Shown instead of the device's name.
                </div>
              </div>
              <input
                key={detail.mac}
                className="prop-input"
                defaultValue={aliases[`bt:${detail.mac}`] ?? ""}
                placeholder={detail.name}
                onBlur={(e) => rename(`bt:${detail.mac}`, e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                }}
              />
            </div>

            {!isWindows && (
              <div className="set-row" style={{ borderBottom: "none" }}>
                <div>
                  <div className="set-label">Auto-connect</div>
                  <div className="panel-sub" style={{ margin: 0 }}>
                    Trust this device so it reconnects automatically.
                  </div>
                </div>
                <button
                  className={`switch ${detail.trusted ? "on" : ""}`}
                  disabled={isBusy(`trust:${detail.mac}`)}
                  onClick={() =>
                    act(
                      `trust:${detail.mac}`,
                      () => api.btSetTrust(detail.mac, !detail.trusted),
                      detail.trusted ? "Auto-connect off" : "Auto-connect on"
                    )
                  }
                />
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
