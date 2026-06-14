import type { Settings } from "../settings";
import { ACCENTS, DEFAULT_SETTINGS, FONT_SCALES, THEMES } from "../settings";

interface Props {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
  platformName?: string;
}

export default function SettingsPanel({ settings, onChange, platformName }: Props) {
  return (
    <div>
      <div className="panel-title">⚙ Settings</div>
      <div className="panel-sub">Personalize how dmgr looks and behaves.</div>

      {/* Theme */}
      <div className="section-h">Appearance</div>
      <div className="card">
        <div style={{ padding: "12px 0", borderBottom: "1px solid var(--bg-alt)" }}>
          <div className="set-label">Theme</div>
          <div className="panel-sub" style={{ margin: "0 0 4px" }}>Full color scheme.</div>
          <div className="theme-grid">
            {THEMES.map((t) => (
              <button
                key={t.id}
                className={`theme-card ${settings.theme === t.id ? "sel" : ""}`}
                onClick={() => onChange({ theme: t.id })}
              >
                <span className="theme-prev" style={{ background: t.bg }}>
                  <span className="dot" style={{ background: t.accent }} />
                </span>
                {t.name}
              </button>
            ))}
          </div>
        </div>

        <div className="set-row">
          <div>
            <div className="set-label">Accent color</div>
            <div className="panel-sub" style={{ margin: 0 }}>
              Used for buttons, highlights and the active selection.
            </div>
          </div>
          <div className="swatches">
            {ACCENTS.map((a) => (
              <button
                key={a.value}
                className={`swatch ${settings.accent.toLowerCase() === a.value.toLowerCase() ? "sel" : ""}`}
                style={{ background: a.value }}
                title={a.name}
                onClick={() => onChange({ accent: a.value })}
              />
            ))}
            <label className="swatch custom" title="Custom color">
              🎨
              <input
                type="color"
                value={settings.accent}
                onChange={(e) => onChange({ accent: e.target.value })}
              />
            </label>
          </div>
        </div>

        <div className="set-row">
          <div>
            <div className="set-label">Interface size</div>
            <div className="panel-sub" style={{ margin: 0 }}>Overall scale of the app.</div>
          </div>
          <div className="seg">
            {FONT_SCALES.map((f) => (
              <button
                key={f.id}
                className={settings.fontScale === f.id ? "active" : ""}
                onClick={() => onChange({ fontScale: f.id })}
              >
                {f.name}
              </button>
            ))}
          </div>
        </div>

        <div className="set-row">
          <div>
            <div className="set-label">Density</div>
            <div className="panel-sub" style={{ margin: 0 }}>Spacing and text size.</div>
          </div>
          <div className="seg">
            <button
              className={settings.density === "comfortable" ? "active" : ""}
              onClick={() => onChange({ density: "comfortable" })}
            >
              Comfortable
            </button>
            <button
              className={settings.density === "compact" ? "active" : ""}
              onClick={() => onChange({ density: "compact" })}
            >
              Compact
            </button>
          </div>
        </div>
      </div>

      {/* Behavior */}
      <div className="section-h">Behavior</div>
      <div className="card">
        <div className="set-row">
          <div>
            <div className="set-label">Remember last view &amp; filters</div>
            <div className="panel-sub" style={{ margin: 0 }}>
              Restore the open panel, “Show all” and Bus/Tree mode next time.
            </div>
          </div>
          <button
            className={`switch ${settings.remember ? "on" : ""}`}
            onClick={() => onChange({ remember: !settings.remember })}
          />
        </div>
      </div>

      <div className="row between" style={{ marginTop: 18 }}>
        <span className="panel-sub" style={{ margin: 0 }}>
          {platformName ? `Running on ${platformName}.` : ""}
        </span>
        <button className="btn ghost" onClick={() => onChange({ ...DEFAULT_SETTINGS })}>
          ↺ Reset to defaults
        </button>
      </div>
    </div>
  );
}
