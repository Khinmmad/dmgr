import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { PhysicalSize } from "@tauri-apps/api/dpi";
import { api } from "./api";
import type { Capabilities, Device, Platform } from "./types";
import { NOISE_BUSES } from "./types";
import Sidebar from "./components/Sidebar";
import type { NavMode } from "./components/Sidebar";
import DeviceDetail from "./components/DeviceDetail";
import AudioPanel from "./components/AudioPanel";
import BluetoothPanel from "./components/BluetoothPanel";
import ModulesPanel from "./components/ModulesPanel";
import SettingsPanel from "./components/SettingsPanel";
import GearIcon from "./components/GearIcon";
import type { Settings } from "./settings";
import {
  applySettings,
  clearUiState,
  loadSettings,
  loadUiState,
  PANEL_META,
  saveSettings,
  saveUiState,
} from "./settings";

export type View = "devices" | "audio" | "bluetooth" | "modules" | "settings";
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
  // One-time read of saved settings + (optionally) last UI state.
  const initial = useMemo(() => {
    const s = loadSettings();
    const ui = s.remember ? loadUiState() : null;
    return { settings: s, ui };
  }, []);

  const [settings, setSettings] = useState<Settings>(initial.settings);
  const [devices, setDevices] = useState<Device[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [showAll, setShowAll] = useState(initial.ui?.showAll ?? false);
  const [navMode, setNavMode] = useState<NavMode>((initial.ui?.navMode as NavMode) ?? "bus");
  const [view, setView] = useState<View>(
    (initial.ui?.view as View) ?? (initial.settings.startupView as View) ?? "devices"
  );
  const [caps, setCaps] = useState<Capabilities | null>(null);
  const [platform, setPlatform] = useState<Platform | null>(null);
  const [toast, setToast] = useState<{ msg: string; kind: "ok" | "err" } | null>(null);

  const notify: Notify = useCallback((msg, kind = "ok") => {
    setToast({ msg, kind });
    window.setTimeout(() => setToast(null), 3200);
  }, []);

  const updateSettings = useCallback((patch: Partial<Settings>) => {
    setSettings((prev) => {
      const next = { ...prev, ...patch };
      saveSettings(next);
      return next;
    });
  }, []);

  // Apply theme / accent / density to the document.
  useEffect(() => {
    applySettings(settings);
  }, [settings]);

  // Remember the window size across sessions.
  useEffect(() => {
    let dispose: (() => void) | undefined;
    let t: number | undefined;
    try {
      const win = getCurrentWindow();
      try {
        const raw = localStorage.getItem("dmgr.win");
        if (raw) {
          const s = JSON.parse(raw);
          if (s && s.w > 300 && s.h > 300) {
            win.setSize(new PhysicalSize(s.w, s.h)).catch(() => {});
          }
        }
      } catch {
        /* ignore */
      }
      win
        .onResized(({ payload }) => {
          window.clearTimeout(t);
          t = window.setTimeout(() => {
            try {
              localStorage.setItem(
                "dmgr.win",
                JSON.stringify({ w: payload.width, h: payload.height })
              );
            } catch {
              /* ignore */
            }
          }, 400);
        })
        .then((fn) => (dispose = fn))
        .catch(() => {});
    } catch {
      /* not running under Tauri */
    }
    return () => {
      dispose?.();
      window.clearTimeout(t);
    };
  }, []);

  // Persist (or forget) the last view & filters per the "remember" preference.
  useEffect(() => {
    if (settings.remember) {
      saveUiState({ view: view === "settings" ? "devices" : view, showAll, navMode });
    } else {
      clearUiState();
    }
  }, [settings.remember, view, showAll, navMode]);

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

  const isLinux = platform?.os === "linux";
  // Audio works everywhere (PipeWire/Pulse/ALSA on Linux, Core Audio on Windows).
  // Bluetooth shows on Linux always, and on Windows only when an adapter exists.
  // Kernel modules are a Linux-only concept.
  const showBluetooth = isLinux || !!caps?.bluetooth;
  const showModules = isLinux;

  // Which panels are available on this platform.
  const panelAvail: Record<string, boolean> = {
    devices: true,
    audio: caps ? caps.audio : true,
    bluetooth: showBluetooth,
    modules: showModules,
  };
  // Nav panels: user-chosen order, minus hidden, minus platform-unavailable.
  const navPanels = settings.panelOrder.filter(
    (id) => panelAvail[id] && !settings.hiddenPanels.includes(id)
  );

  // If the current view is unavailable or the user hid it, fall back to the
  // first visible panel (Settings is always reachable via the gear button).
  useEffect(() => {
    const ok =
      view === "settings" || (panelAvail[view] && !settings.hiddenPanels.includes(view));
    if (!ok) setView((navPanels[0] as View) ?? "settings");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [view, caps, platform, showBluetooth, showModules, settings.hiddenPanels, settings.panelOrder]);

  return (
    <div className="app">
      <header className="topbar">
        <span className="brand"><GearIcon size={16} /> dmgr</span>
        {navPanels.map((id) => {
          const meta = PANEL_META.find((p) => p.id === id);
          if (!meta) return null;
          return (
            <button
              key={id}
              className={`iconbtn ${view === id ? "active" : ""}`}
              onClick={() => setView(id as View)}
            >
              {meta.label}
            </button>
          );
        })}
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
        {view === "devices" && (
          <button className="iconbtn" onClick={refresh} title="Rescan">
            ⟳
          </button>
        )}
        <button
          className={`iconbtn ${view === "settings" ? "active" : ""}`}
          onClick={() => setView("settings")}
          title="Settings"
        >
          <GearIcon size={14} />
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
            <DeviceDetail
              device={selected}
              os={platform?.os ?? "linux"}
              notify={notify}
              onChanged={refresh}
            />
          ) : (
            <div className="empty">
              Select a device from the left to view and manage it.
              <br />
              {devices.length} devices detected · {visible.length} shown
            </div>
          ))}

        {view === "audio" && (
          <AudioPanel notify={notify} notifications={settings.notifications} />
        )}
        {view === "bluetooth" && (
          <BluetoothPanel
            notify={notify}
            os={platform?.os ?? "linux"}
            notifications={settings.notifications}
          />
        )}
        {view === "modules" && <ModulesPanel notify={notify} />}
        {view === "settings" && (
          <SettingsPanel
            settings={settings}
            onChange={updateSettings}
            platformName={platform?.distro_name}
          />
        )}
      </main>

      {platform && (
        <footer className="statusbar">
          <span title={platform.distro_name}>
            {platform.os === "windows" ? "🪟" : platform.os === "macos" ? "🍎" : "🐧"}{" "}
            {platform.os === "windows" ? platform.distro_name : platform.distro_id}
            {platform.session ? ` · ${platform.session}` : ""}
            {platform.gpu_nvidia ? " · nvidia" : ""}
          </span>
          {platform.audio_backend && platform.audio_backend !== "none" && (
            <span>· 🔊 {platform.audio_backend}</span>
          )}
          {!platform.can_elevate && (
            <span
              className="warn"
              title={
                platform.os === "windows"
                  ? "Not running as Administrator. Enable/Disable will prompt for elevation (UAC)."
                  : `Privileged actions unavailable. Install dmgr-polkit-helper + polkit. ${platform.package_hint}`
              }
            >
              · ⚠ {platform.os === "windows" ? "not admin" : "no root"}
            </span>
          )}
        </footer>
      )}

      {toast && <div className={`toast ${toast.kind}`}>{toast.msg}</div>}
    </div>
  );
}
