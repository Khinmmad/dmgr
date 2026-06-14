import { useCallback, useEffect, useState } from "react";
import type { Notify } from "../App";
import { api } from "../api";
import type { BtDevice, BtState } from "../types";

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
  // Per-action in-flight guard. Prevents overlapping bluetoothctl calls when
  // the user double-clicks or rapidly toggles. Keys: "power", "scan",
  // "trust:<mac>", "connect:<mac>", "disconnect:<mac>", "remove:<mac>",
  // "pair:<mac>".
  const [inFlight, setInFlight] = useState<Set<string>>(new Set());
  // True while a scan is running, for the spinner.
  const [scanning, setScanning] = useState(false);

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
  const byPreference = (a: BtDevice, b: BtDevice) =>
    Number(b.connected) - Number(a.connected) || a.name.localeCompare(b.name);
  const paired = devices.filter((d) => d.paired).sort(byPreference);
  // Discovered = seen during a scan but not yet paired (Linux only; on Windows
  // every listed device is already paired and discovery is the OS's job).
  const discovered = devices.filter((d) => !d.paired).sort(byPreference);

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
                disabled={isBusy(`trust:${d.mac}`)}
                onClick={() =>
                  act(
                    `trust:${d.mac}`,
                    () => api.btSetTrust(d.mac, !d.trusted),
                    d.trusted ? "Untrusted" : "Trusted"
                  )
                }
                title={d.trusted ? "Untrust this device" : "Trust this device"}
              >
                {d.trusted ? "Trusted" : "Trust"}
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
                <div className="name">{d.name}</div>
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
    </div>
  );
}
