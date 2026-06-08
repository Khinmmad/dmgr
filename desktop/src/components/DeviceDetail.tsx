import { useEffect, useState } from "react";
import type { Notify } from "../App";
import { api } from "../api";
import type { DetailItem, Device } from "../types";
import { BUS_META, STATUS_META } from "../types";
import PropertyTable from "./PropertyTable";

interface Props {
  device: Device;
  os: string;
  notify: Notify;
  onChanged: () => void;
}

export default function DeviceDetail({ device, os, notify, onChanged }: Props) {
  const [drivers, setDrivers] = useState<string[]>([]);
  const [pick, setPick] = useState("");
  const [busy, setBusy] = useState(false);

  const isWindows = os === "windows";
  // Linux exposes the kernel `authorized` flag for USB/PCI only; Windows can
  // enable/disable almost any PnP device via Enable/Disable-PnpDevice.
  const canToggle = isWindows
    ? device.bus !== "Unknown"
    : device.bus === "Usb" || device.bus === "Pci";

  useEffect(() => {
    setPick("");
    api.availableDrivers(device.path).then(setDrivers).catch(() => setDrivers([]));
  }, [device.path]);

  const guard = async (fn: () => Promise<void>, okMsg: string) => {
    setBusy(true);
    try {
      await fn();
      notify(okMsg, "ok");
      onChanged();
    } catch (e) {
      notify(String(e), "err");
    } finally {
      setBusy(false);
    }
  };

  const status = STATUS_META[device.status];

  return (
    <div>
      <div className="row between">
        <div>
          <div className="panel-title">{device.name}</div>
          <div className="panel-sub">
            {BUS_META[device.bus].icon} {BUS_META[device.bus].label}
            {device.driver ? ` · driver: ${device.driver}` : " · no driver"}
          </div>
        </div>
        <span
          className="pill"
          style={{ background: status.color, color: "#1e1e2e" }}
        >
          {status.dot} {status.label}
        </span>
      </div>

      {/* Identity */}
      <div className="card">
        <table className="prop-table">
          <tbody>
            <Info k="Vendor" v={fmt(device.vendor, device.vendor_id)} />
            <Info k="Model" v={fmt(device.model, device.model_id)} />
            <Info k="Bus ID" v={device.bus_id} />
            <Info k="Subsystem" v={device.subsystem} />
            <Info k="Driver" v={device.driver} />
            <Info k={isWindows ? "Instance ID" : "Sysfs path"} v={device.path} />
            <Info k="Removable" v={device.removable ? "yes" : "no"} />
          </tbody>
        </table>
      </div>

      {/* Actions — Windows Device Manager style */}
      <div className="section-h">Manage device</div>
      <div className="card">
        {canToggle && (
          <div className="row between" style={{ marginBottom: 14 }}>
            <div>
              <div style={{ fontWeight: 600 }}>
                {device.authorized ? "Device enabled" : "Device disabled"}
              </div>
              <div className="panel-sub" style={{ margin: 0 }}>
                {isWindows ? (
                  <>Enables or disables the device (may prompt for Administrator).</>
                ) : (
                  <>Toggles the kernel <code>authorized</code> flag (needs root).</>
                )}
              </div>
            </div>
            <button
              className={`switch ${device.authorized ? "on" : ""}`}
              disabled={busy}
              onClick={() =>
                guard(
                  () => api.setDeviceEnabled(device.path, !device.authorized),
                  device.authorized ? "Device disabled" : "Device enabled"
                )
              }
            />
          </div>
        )}

        <div className="row" style={{ flexWrap: "wrap", gap: 10 }}>
          {device.driver ? (
            <button
              className="btn danger"
              disabled={busy}
              onClick={() => {
                if (!confirm(`Uninstall driver "${device.driver}" from ${device.name}?`))
                  return;
                guard(() => api.unbindDriver(device.path), "Driver unbound");
              }}
            >
              ✕ Uninstall driver
            </button>
          ) : (
            <span className="panel-sub" style={{ margin: 0 }}>
              No driver bound.
            </span>
          )}

          {drivers.length > 0 && (
            <div className="row" style={{ gap: 8 }}>
              <select
                className="prop-input"
                value={pick}
                onChange={(e) => setPick(e.target.value)}
                style={{ width: 200 }}
              >
                <option value="">Select driver…</option>
                {drivers.map((d) => (
                  <option key={d} value={d}>
                    {d}
                  </option>
                ))}
              </select>
              <button
                className="btn primary"
                disabled={busy || !pick}
                onClick={() =>
                  guard(() => api.bindDriver(device.path, pick), `Bound to ${pick}`)
                }
              >
                ⟳ {device.driver ? "Change" : "Install"} driver
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Advanced details (lazy) */}
      <AdvancedSection path={device.path} bus={device.bus} />

      {/* Properties */}
      <div className="section-h">Properties</div>
      <PropertyTable device={device} notify={notify} onChanged={onChanged} />
    </div>
  );
}

function AdvancedSection({ path, bus }: { path: string; bus: string }) {
  const [open, setOpen] = useState(false);
  const [items, setItems] = useState<DetailItem[] | null>(null);

  useEffect(() => {
    setItems(null);
    setOpen(false);
  }, [path]);

  const toggle = () => {
    const next = !open;
    setOpen(next);
    if (next && items === null) {
      api.advancedDetails(path, bus).then(setItems).catch(() => setItems([]));
    }
  };

  return (
    <>
      <button
        className="section-h"
        style={{ background: "none", cursor: "pointer", display: "flex", gap: 6 }}
        onClick={toggle}
      >
        <span style={{ color: "var(--overlay)" }}>{open ? "▾" : "▸"}</span>
        Advanced details
      </button>
      {open && (
        <div className="card">
          {items === null ? (
            <div className="panel-sub">Reading sysfs…</div>
          ) : items.length === 0 ? (
            <div className="panel-sub">No advanced details for this device.</div>
          ) : (
            <table className="prop-table">
              <tbody>
                {items.map((it) => (
                  <tr key={it.label}>
                    <td className="k">{it.label}</td>
                    <td className="v">{it.value}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}
    </>
  );
}

function Info({ k, v }: { k: string; v: string | null | undefined }) {
  if (!v) return null;
  return (
    <tr>
      <td className="k">{k}</td>
      <td className="v">{v}</td>
    </tr>
  );
}

function fmt(name: string | null, id: string | null): string | null {
  if (name && id) return `${name} [${id}]`;
  return name || id || null;
}
