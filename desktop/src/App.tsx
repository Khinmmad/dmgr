import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import type { Capabilities, Device, Platform } from "./types";
import { NOISE_BUSES } from "./types";
import Sidebar from "./components/Sidebar";
import type { NavMode } from "./components/Sidebar";
import DeviceDetail from "./components/DeviceDetail";
import AudioPanel from "./components/AudioPanel";
import BluetoothPanel from "./components/BluetoothPanel";

export type View = "devices" | "audio" | "bluetooth";
export type Notify = (msg: string, kind?: "ok" | "err") => void;

// Internal kernel sub-nodes that aren't user-facing "devices":
//  - ALSA: controlC0 / pcmC0D3p / hwC0D0 / midiC* / seq / timer  (keep "Sound Card: X")
//  - DRM:  cardN-DP-*, cardN-HDMI-*, cardN-Writeback-*, renderD*, version  (keep cardN)
const SOUND_NOISE = /^sound Device /;
const DRM_NOISE = /^drm Device (card\d+-|renderD|controlD|version)/;

/** A device is "relevant" when it is a real, active or actionable device. */
export function isRelevant(d: Device): boolean {
  if (NOISE_BUSES.includes(d.bus)) return false;
  if (d.bus === "Audio" && SOUND_NOISE.test(d.name)) return false;
  if (d.bus === "Drm" && DRM_NOISE.test(d.name)) return false;
  const active = d.status === "Online" || d.status === "Suspended";
  const actionable = d.removable || d.editable_properties.length > 0 || !!d.driver;
  return active || actionable;
}

export default function App() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [showAll, setShowAll] = useState(false);
  const [navMode, setNavMode] = useState<NavMode>("bus");
  const [view, setView] = useState<View>("devices");
  const [caps, setCaps] = useState<Capabilities | null>(null);
  const [platform, setPlatform] = useState<Platform | null>(null);
  const [toast, setToast] = useState<{ msg: string; kind: "ok" | "err" } | null>(null);

  const notify: Notify = useCallback((msg, kind = "ok") => {
    setToast({ msg, kind });
    window.setTimeout(() => setToast(null), 3200);
  }, []);

  const refresh = useCallback(async () => {
    try {
      const list = await api.scanDevices();
      list.sort((a, b) => a.name.localeCompare(b.name));
      setDevices(list);
    } catch (e) {
      notify(String(e), "err");
    } finally {
      setLoading(false);
    }
  }, [notify]);

  useEffect(() => {
    api.capabilities().then(setCaps).catch(() => {});
    api.platformInfo().then(setPlatform).catch(() => {});
    refresh();
  }, [refresh]);

  // Live hotplug: backend emits `devices-changed`; debounce bursts then re-scan.
  const debounceRef = useRef<number | undefined>(undefined);
  useEffect(() => {
    const unlisten = listen("devices-changed", () => {
      window.clearTimeout(debounceRef.current);
      debounceRef.current = window.setTimeout(refresh, 400);
    });
    return () => {
      unlisten.then((fn) => fn());
      window.clearTimeout(debounceRef.current);
    };
  }, [refresh]);

  const visible = useMemo(() => {
    const q = search.trim().toLowerCase();
    return devices.filter((d) => {
      if (!showAll && !isRelevant(d)) return false;
      if (!q) return true;
      return (
        d.name.toLowerCase().includes(q) ||
        (d.vendor ?? "").toLowerCase().includes(q) ||
        (d.driver ?? "").toLowerCase().includes(q) ||
        d.subsystem.toLowerCase().includes(q)
      );
    });
  }, [devices, search, showAll, view]);

  const selected = devices.find((d) => d.id === selectedId) ?? null;

  return (
    <div className="app">
      <header className="topbar">
        <span className="brand">⚙ dmgr</span>
        <button
          className={`iconbtn ${view === "devices" ? "active" : ""}`}
          onClick={() => setView("devices")}
        >
          Devices
        </button>
        <button
          className={`iconbtn ${view === "audio" ? "active" : ""}`}
          onClick={() => setView("audio")}
          disabled={caps ? !caps.audio : false}
          title={caps && !caps.audio ? "pactl not available" : ""}
        >
          🔊 Audio
        </button>
        <button
          className={`iconbtn ${view === "bluetooth" ? "active" : ""}`}
          onClick={() => setView("bluetooth")}
        >
          🔵 Bluetooth
        </button>
        <span className="spacer" />
        {view === "devices" && (
          <>
            <input
              className="search"
              placeholder="Search devices…"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
            <button
              className={`iconbtn ${navMode === "bus" ? "active" : ""}`}
              onClick={() => setNavMode("bus")}
              title="Group by bus"
            >
              Bus
            </button>
            <button
              className={`iconbtn ${navMode === "tree" ? "active" : ""}`}
              onClick={() => setNavMode("tree")}
              title="Hierarchy (parent / child)"
            >
              Tree
            </button>
            <label className="toggle">
              <input
                type="checkbox"
                checked={showAll}
                onChange={(e) => setShowAll(e.target.checked)}
              />
              Show all
            </label>
          </>
        )}
        <button className="iconbtn" onClick={refresh} title="Rescan">
          ⟳
        </button>
      </header>

      <Sidebar
        view={view}
        mode={navMode}
        devices={visible}
        total={devices.length}
        selectedId={selectedId}
        onSelect={(id) => {
          setSelectedId(id);
          setView("devices");
        }}
      />

      <main className="content">
        {view === "devices" &&
          (loading ? (
            <div className="spinner">Scanning devices…</div>
          ) : selected ? (
            <DeviceDetail device={selected} notify={notify} onChanged={refresh} />
          ) : (
            <div className="empty">
              Select a device from the left to view and manage it.
              <br />
              {devices.length} devices detected · {visible.length} shown
            </div>
          ))}

        {view === "audio" && <AudioPanel notify={notify} />}
        {view === "bluetooth" && <BluetoothPanel notify={notify} />}
      </main>

      {platform && (
        <footer className="statusbar">
          <span title={platform.distro_name}>
            🐧 {platform.distro_id} · {platform.session}
            {platform.gpu_nvidia ? " · nvidia" : ""}
          </span>
          <span>· 🔊 {platform.audio_backend}</span>
          {!platform.can_elevate && (
            <span
              className="warn"
              title={`Privileged actions unavailable. Install dmgr-polkit-helper + polkit. ${platform.package_hint}`}
            >
              · ⚠ no root
            </span>
          )}
        </footer>
      )}

      {toast && <div className={`toast ${toast.kind}`}>{toast.msg}</div>}
    </div>
  );
}
