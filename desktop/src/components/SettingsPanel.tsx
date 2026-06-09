import type { Settings } from "../settings";
import { ACCENTS } from "../settings";

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
        <div className="set-row">
          <div>
            <div className="set-label">Theme</div>
            <div className="panel-sub" style={{ margin: 0 }}>Light or dark color scheme.</div>
          </div>
          <div className="seg">
            <button
              className={settings.theme === "dark" ? "active" : ""}
              onClick={() => onChange({ theme: "dark" })}
            >
              🌙 Dark
            </button>
            <button
              className={settings.theme === "light" ? "active" : ""}
              onClick={() => onChange({ theme: "light" })}
            >
              ☀ Light
            </button>
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

      {platformName && (
        <div className="panel-sub" style={{ marginTop: 18 }}>
          Running on {platformName}.
        </div>
      )}
    </div>
  );
}
