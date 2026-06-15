import type { Settings } from "../settings";
import {
  ACCENTS,
  DEFAULT_SETTINGS,
  effectivePanelOrder,
  FONT_SCALES,
  PANEL_META,
  THEMES,
} from "../settings";

interface Props {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
  platformName?: string;
}

export default function SettingsPanel({ settings, onChange, platformName }: Props) {
  const panelOrder = effectivePanelOrder(settings.panelOrder);
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

        <div className="set-row">
          <div>
            <div className="set-label">Reduce motion</div>
            <div className="panel-sub" style={{ margin: 0 }}>
              Disable transitions and animations.
            </div>
          </div>
          <button
            className={`switch ${settings.disableAnimations ? "on" : ""}`}
            onClick={() => onChange({ disableAnimations: !settings.disableAnimations })}
          />
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

        <div className="set-row">
          <div>
            <div className="set-label">Device notifications</div>
            <div className="panel-sub" style={{ margin: 0 }}>
              Show a toast when a Bluetooth or audio device connects or disconnects.
            </div>
          </div>
          <button
            className={`switch ${settings.notifications ? "on" : ""}`}
            onClick={() => onChange({ notifications: !settings.notifications })}
          />
        </div>

        <div className="set-row">
          <div>
            <div className="set-label">Confirm destructive actions</div>
            <div className="panel-sub" style={{ margin: 0 }}>
              Ask before unbinding a driver, unpairing, or stopping a service.
            </div>
          </div>
          <button
            className={`switch ${settings.confirmDestructive ? "on" : ""}`}
            onClick={() => onChange({ confirmDestructive: !settings.confirmDestructive })}
          />
        </div>
      </div>

      {/* Panels */}
      <div className="section-h">Panels</div>
      <div className="card">
        <div className="set-row">
          <div>
            <div className="set-label">Startup panel</div>
            <div className="panel-sub" style={{ margin: 0 }}>
              Which panel opens when dmgr launches.
            </div>
          </div>
          <select
            className="prop-input"
            value={settings.startupView}
            onChange={(e) => onChange({ startupView: e.target.value })}
          >
            {PANEL_META.map((p) => (
              <option key={p.id} value={p.id}>
                {p.label}
              </option>
            ))}
            <option value="settings">Settings</option>
          </select>
        </div>

        {panelOrder.map((id, i) => {
          const meta = PANEL_META.find((p) => p.id === id);
          if (!meta) return null;
          const hidden = settings.hiddenPanels.includes(id);
          const move = (dir: number) => {
            const arr = [...panelOrder];
            const j = i + dir;
            if (j < 0 || j >= arr.length) return;
            [arr[i], arr[j]] = [arr[j], arr[i]];
            onChange({ panelOrder: arr });
          };
          const toggleHidden = () => {
            const set = new Set(settings.hiddenPanels);
            if (set.has(id)) set.delete(id);
            else set.add(id);
            onChange({ hiddenPanels: [...set] });
          };
          return (
            <div className="set-row" key={id}>
              <div className="set-label">{meta.label}</div>
              <div className="row" style={{ gap: 6 }}>
                <button
                  className="btn ghost"
                  onClick={() => move(-1)}
                  disabled={i === 0}
                  title="Move up"
                >
                  ↑
                </button>
                <button
                  className="btn ghost"
                  onClick={() => move(1)}
                  disabled={i === panelOrder.length - 1}
                  title="Move down"
                >
                  ↓
                </button>
                <button
                  className={`switch ${!hidden ? "on" : ""}`}
                  onClick={toggleHidden}
                  title={hidden ? "Show in nav" : "Hide from nav"}
                />
              </div>
            </div>
          );
        })}
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
