import { useCallback, useEffect, useState } from "react";
import type { Notify } from "../App";
import { api } from "../api";
import { useAliases } from "../aliases";
import type { AudioDevice } from "../types";

const KIND_ICON: Record<AudioDevice["kind"], string> = {
  Bluetooth: "🔵",
  Hdmi: "🖥",
  Usb: "🔌",
  Virtual: "⬡",
  Builtin: "🔊",
};

interface Props {
  notify: Notify;
}

export default function AudioPanel({ notify }: Props) {
  const [outputs, setOutputs] = useState<AudioDevice[]>([]);
  const [inputs, setInputs] = useState<AudioDevice[]>([]);
  const [busy, setBusy] = useState(false);
  const [editKey, setEditKey] = useState<string | null>(null);
  const { map: aliases, name: aliasName, rename } = useAliases();

  const load = useCallback(async () => {
    try {
      const [o, i] = await Promise.all([api.audioOutputs(), api.audioInputs()]);
      setOutputs(o);
      setInputs(i);
    } catch (e) {
      notify(String(e), "err");
    }
  }, [notify]);

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

      <div className="section-h">Output devices</div>
      {outputs.length === 0 && <div className="empty">No output devices found.</div>}
      {outputs.map((d) => (
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

      {inputs.length > 0 && (
        <>
          <div className="section-h">Input devices</div>
          {inputs.map((d) => (
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
    </div>
  );
}
