import { useCallback, useEffect, useState } from "react";
import type { Notify } from "../App";
import { api } from "../api";
import type { PowerInfo } from "../types";

const PROFILE_LABEL: Record<string, string> = {
  "power-saver": "🍃 Power saver",
  balanced: "Balanced",
  performance: "🚀 Performance",
};

interface Props {
  notify: Notify;
}

export default function PowerPanel({ notify }: Props) {
  const [info, setInfo] = useState<PowerInfo | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      setInfo(await api.powerInfo());
    } catch (e) {
      notify(String(e), "err");
    }
  }, [notify]);

  useEffect(() => {
    load();
    const t = window.setInterval(load, 5000);
    return () => window.clearInterval(t);
  }, [load]);

  const setProfile = async (p: string) => {
    setBusy(true);
    try {
      await api.setPowerProfile(p);
      notify(`Power profile → ${p}`, "ok");
      await load();
    } catch (e) {
      notify(String(e), "err");
    } finally {
      setBusy(false);
    }
  };

  const onBrightness = async (pct: number) => {
    setInfo((prev) => (prev ? { ...prev, brightness_percent: pct } : prev));
    try {
      await api.setBrightness(pct);
    } catch (e) {
      notify(String(e), "err");
    }
  };

  if (!info) return <div className="spinner">Reading power info…</div>;

  const nothing = !info.has_ppd && info.battery_percent == null && !info.has_brightness;

  return (
    <div>
      <div className="row between">
        <div>
          <div className="panel-title">🔋 Power</div>
          <div className="panel-sub">Power profile, battery and screen brightness.</div>
        </div>
        <button className="iconbtn" onClick={load} title="Refresh">
          ⟳
        </button>
      </div>

      {nothing && (
        <div className="empty">
          No power-management features detected. Install <code>power-profiles-daemon</code> for
          profiles, or <code>brightnessctl</code> for brightness control.
        </div>
      )}

      {info.has_ppd && (
        <>
          <div className="section-h">Power profile</div>
          <div className="card">
            <div className="seg">
              {info.profiles.map((p) => (
                <button
                  key={p}
                  className={info.active_profile === p ? "active" : ""}
                  disabled={busy}
                  onClick={() => setProfile(p)}
                >
                  {PROFILE_LABEL[p] ?? p}
                </button>
              ))}
            </div>
          </div>
        </>
      )}

      {info.battery_percent != null && (
        <>
          <div className="section-h">Battery</div>
          <div className="card">
            <div className="row between" style={{ marginBottom: 8 }}>
              <span className="set-label">{info.battery_status || "Battery"}</span>
              <span className="panel-sub" style={{ margin: 0 }}>
                {info.battery_percent}%
              </span>
            </div>
            <div className="meter">
              <div
                className="meter-fill"
                style={{
                  width: `${info.battery_percent}%`,
                  background: info.battery_percent <= 15 ? "var(--red)" : "var(--green)",
                }}
              />
            </div>
          </div>
        </>
      )}

      {info.has_brightness && info.brightness_percent != null && (
        <>
          <div className="section-h">Brightness</div>
          <div className="card">
            <div className="row" style={{ gap: 14 }}>
              <span style={{ fontSize: 18 }}>🔆</span>
              <input
                className="slider"
                style={{ flex: 1, width: "auto" }}
                type="range"
                min={5}
                max={100}
                value={info.brightness_percent}
                onChange={(e) => onBrightness(Number(e.target.value))}
              />
              <span className="panel-sub" style={{ margin: 0, width: 42, textAlign: "right" }}>
                {info.brightness_percent}%
              </span>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
