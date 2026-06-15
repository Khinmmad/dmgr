import { useCallback, useEffect, useRef, useState } from "react";
import type { Notify } from "../App";
import { api } from "../api";
import { useAliases } from "../aliases";
import { useFavorites } from "../favorites";
import type { AudioApp, AudioDevice } from "../types";

const KIND_ICON: Record<AudioDevice["kind"], string> = {
  Bluetooth: "🔵",
  Hdmi: "🖥",
  Usb: "🔌",
  Virtual: "⬡",
  Builtin: "🔊",
};

interface Props {
  notify: Notify;
  notifications: boolean;
}

export default function AudioPanel({ notify, notifications }: Props) {
  const [outputs, setOutputs] = useState<AudioDevice[]>([]);
  const [inputs, setInputs] = useState<AudioDevice[]>([]);
  const [apps, setApps] = useState<AudioApp[]>([]);
  const [busy, setBusy] = useState(false);
  const [editKey, setEditKey] = useState<string | null>(null);
  const [q, setQ] = useState("");
  const { map: aliases, name: aliasName, rename } = useAliases();
  const { has: isFav, toggle: toggleFav } = useFavorites();

  // Previous output devices (name → description), to detect appear/disappear.
  const prevOut = useRef<Map<string, string> | null>(null);

  const load = useCallback(async () => {
    try {
      const [o, i, a] = await Promise.all([
        api.audioOutputs(),
        api.audioInputs(),
        api.audioAppStreams(),
      ]);
      const cur = new Map(o.map((d) => [d.name, d.description] as [string, string]));
      if (prevOut.current && notifications) {
        for (const [name, desc] of cur)
          if (!prevOut.current.has(name)) notify(`${desc} available`, "ok");
        for (const [name, desc] of prevOut.current)
          if (!cur.has(name)) notify(`${desc} removed`, "ok");
      }
      prevOut.current = cur;
      setOutputs(o);
      setInputs(i);
      setApps(a);
    } catch (e) {
      notify(String(e), "err");
    }
  }, [notify, notifications]);

  useEffect(() => {
    load();
    const t = window.setInterval(load, 4000);
    return () => window.clearInterval(t);
  }, [load]);

  const switchOutput = async (d: AudioDevice) => {
    if (d.is_default) return;
    setBusy(true);
    try {
      await api.setDefaultOutput(d.name);
      notify(`Output → ${d.description}`, "ok");
      await load();
    } catch (e) {
      notify(String(e), "err");
    } finally {
      setBusy(false);
    }
  };

  const switchInput = async (d: AudioDevice) => {
    if (d.is_default) return;
    try {
      await api.setDefaultInput(d.name);
      notify(`Input → ${d.description}`, "ok");
      await load();
    } catch (e) {
      notify(String(e), "err");
    }
  };

  const onVolume = async (d: AudioDevice, percent: number) => {
    setOutputs((prev) =>
      prev.map((x) => (x.name === d.name ? { ...x, volume: percent } : x))
    );
    try {
      await api.setVolume(d.name, percent);
    } catch (e) {
      notify(String(e), "err");
    }
  };

  const toggleMute = async (d: AudioDevice) => {
    try {
      await api.setMute(d.name, !d.muted);
      await load();
    } catch (e) {
      notify(String(e), "err");
    }
  };

  const onAppVolume = async (a: AudioApp, percent: number) => {
    setApps((prev) => prev.map((x) => (x.index === a.index ? { ...x, volume: percent } : x)));
    try {
      await api.setAppVolume(a.index, percent);
    } catch (e) {
      notify(String(e), "err");
    }
  };

  const toggleAppMute = async (a: AudioApp) => {
    try {
      await api.setAppMute(a.index, !a.muted);
      await load();
    } catch (e) {
      notify(String(e), "err");
    }
  };

  const ql = q.trim().toLowerCase();
  const an = (d: AudioDevice) => aliasName(`audio:${d.name}`, d.description);
  const order = (list: AudioDevice[]) =>
    list
      .filter((d) => !ql || an(d).toLowerCase().includes(ql))
      .sort(
        (a, b) =>
          Number(isFav(`audio:${b.name}`)) - Number(isFav(`audio:${a.name}`)) ||
          an(a).localeCompare(an(b))
      );
  const outs = order(outputs);
  const ins = order(inputs);

  return (
    <div>
      <div className="row between">
        <div>
          <div className="panel-title">🔊 Audio</div>
          <div className="panel-sub">
            Choose where sound plays. The active output is highlighted in green.
          </div>
        </div>
        <button className="iconbtn" onClick={load} title="Refresh">
          ⟳
        </button>
      </div>

      <input
        className="search"
        style={{ width: "100%", margin: "0 0 8px" }}
        placeholder="Filter audio devices…"
        value={q}
        onChange={(e) => setQ(e.target.value)}
      />

      <div className="section-h">Output devices</div>
      {outs.length === 0 && <div className="empty">No output devices found.</div>}
      {outs.map((d) => (
        <div key={d.name} className={`media-item ${d.is_default ? "active" : ""}`}>
          <span className="ico">{KIND_ICON[d.kind]}</span>
          <div className="meta">
            {editKey === `audio:${d.name}` ? (
              <input
                className="prop-input"
                autoFocus
                defaultValue={aliases[`audio:${d.name}`] ?? ""}
                placeholder={d.description}
                onBlur={(e) => {
                  rename(`audio:${d.name}`, e.target.value);
                  setEditKey(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                  if (e.key === "Escape") setEditKey(null);
                }}
              />
            ) : (
              <div
                className="name"
                title="Double-click to rename"
                onDoubleClick={() => setEditKey(`audio:${d.name}`)}
              >
                {aliasName(`audio:${d.name}`, d.description)}
              </div>
            )}
            <div className="desc">
              {d.kind} · {d.state || "—"}
              {d.volume != null ? ` · ${d.volume}%` : ""}
            </div>
          </div>

          <button
            className="btn ghost"
            style={{ opacity: isFav(`audio:${d.name}`) ? 1 : 0.35, padding: "8px 10px" }}
            onClick={() => toggleFav(`audio:${d.name}`)}
            title={isFav(`audio:${d.name}`) ? "Unpin" : "Pin to top"}
          >
            ⭐
          </button>

          <button
            className="btn ghost"
            onClick={() => toggleMute(d)}
            title={d.muted ? "Unmute" : "Mute"}
          >
            {d.muted ? "🔇" : "🔊"}
          </button>

          {d.volume != null && (
            <input
              className="slider"
              type="range"
              min={0}
              max={100}
              value={d.volume}
              onChange={(e) => onVolume(d, Number(e.target.value))}
            />
          )}

          {d.is_default ? (
            <span className="badge-active">● Active</span>
          ) : (
            <button
              className="btn primary"
              disabled={busy}
              onClick={() => switchOutput(d)}
            >
              Use this
            </button>
          )}
        </div>
      ))}

      {ins.length > 0 && (
        <>
          <div className="section-h">Input devices</div>
          {ins.map((d) => (
            <div key={d.name} className={`media-item ${d.is_default ? "active" : ""}`}>
              <span className="ico">🎙</span>
              <div className="meta">
                {editKey === `audio:${d.name}` ? (
              <input
                className="prop-input"
                autoFocus
                defaultValue={aliases[`audio:${d.name}`] ?? ""}
                placeholder={d.description}
                onBlur={(e) => {
                  rename(`audio:${d.name}`, e.target.value);
                  setEditKey(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                  if (e.key === "Escape") setEditKey(null);
                }}
              />
            ) : (
              <div
                className="name"
                title="Double-click to rename"
                onDoubleClick={() => setEditKey(`audio:${d.name}`)}
              >
                {aliasName(`audio:${d.name}`, d.description)}
              </div>
            )}
                <div className="desc">
                  {d.kind} · {d.state || "—"}
                </div>
              </div>
              <button
                className="btn ghost"
                style={{ opacity: isFav(`audio:${d.name}`) ? 1 : 0.35, padding: "8px 10px" }}
                onClick={() => toggleFav(`audio:${d.name}`)}
                title={isFav(`audio:${d.name}`) ? "Unpin" : "Pin to top"}
              >
                ⭐
              </button>
              {d.is_default ? (
                <span className="badge-active">● Active</span>
              ) : (
                <button className="btn primary" onClick={() => switchInput(d)}>
                  Use this
                </button>
              )}
            </div>
          ))}
        </>
      )}

      {apps.length > 0 && (
        <>
          <div className="section-h">Applications</div>
          {apps.map((a) => (
            <div key={a.index} className="media-item">
              <span className="ico">🎵</span>
              <div className="meta">
                <div className="name">{a.name}</div>
                <div className="desc">
                  {a.media && a.media !== a.name ? a.media : "playing"}
                  {a.volume != null ? ` · ${a.volume}%` : ""}
                </div>
              </div>
              <button
                className="btn ghost"
                onClick={() => toggleAppMute(a)}
                title={a.muted ? "Unmute" : "Mute"}
              >
                {a.muted ? "🔇" : "🔊"}
              </button>
              {a.volume != null && (
                <input
                  className="slider"
                  type="range"
                  min={0}
                  max={100}
                  value={a.volume}
                  onChange={(e) => onAppVolume(a, Number(e.target.value))}
                />
              )}
            </div>
          ))}
        </>
      )}
    </div>
  );
}
