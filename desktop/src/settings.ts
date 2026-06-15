// User customization: theme, accent color, density — persisted in localStorage
// and applied via attributes/CSS variables on <html>.

export type Theme = "dark" | "light" | "macchiato" | "nord" | "gruvbox" | "dracula";
export type Density = "comfortable" | "compact";
export type FontScale = "sm" | "md" | "lg";

export interface Settings {
  theme: Theme;
  accent: string; // hex color
  density: Density;
  fontScale: FontScale; // overall interface scale (zoom)
  remember: boolean; // remember last view & filters across sessions
  notifications: boolean; // toast on device connect/disconnect
  panelOrder: string[]; // nav panel ids, in display order
  hiddenPanels: string[]; // panel ids hidden from the nav
  startupView: string; // panel id to open on launch
}

export interface UiState {
  view: string;
  showAll: boolean;
  navMode: string;
}

export const ACCENTS: { name: string; value: string }[] = [
  { name: "Blue", value: "#89b4fa" },
  { name: "Mauve", value: "#cba6f7" },
  { name: "Green", value: "#a6e3a1" },
  { name: "Peach", value: "#fab387" },
  { name: "Red", value: "#f38ba8" },
  { name: "Teal", value: "#94e2d5" },
];

// Full color-scheme presets. `bg`/`accent` are only for the picker preview;
// the actual palettes live in styles.css under `:root[data-theme="…"]`.
// "dark" = Catppuccin Mocha (the base `:root`), "light" = Catppuccin Latte.
export const THEMES: { id: Theme; name: string; bg: string; accent: string }[] = [
  { id: "dark", name: "Mocha", bg: "#1e1e2e", accent: "#89b4fa" },
  { id: "macchiato", name: "Macchiato", bg: "#24273a", accent: "#8aadf4" },
  { id: "nord", name: "Nord", bg: "#2e3440", accent: "#88c0d0" },
  { id: "gruvbox", name: "Gruvbox", bg: "#282828", accent: "#fabd2f" },
  { id: "dracula", name: "Dracula", bg: "#282a36", accent: "#bd93f9" },
  { id: "light", name: "Latte", bg: "#eff1f5", accent: "#1e66f5" },
];

export const FONT_SCALES: { id: FontScale; name: string; zoom: number }[] = [
  { id: "sm", name: "Small", zoom: 0.9 },
  { id: "md", name: "Normal", zoom: 1 },
  { id: "lg", name: "Large", zoom: 1.12 },
];

// Configurable nav panels (Settings is always reachable via the gear button and
// isn't listed here). `id` matches the App `View` union.
export const PANEL_META: { id: string; label: string }[] = [
  { id: "devices", label: "Devices" },
  { id: "audio", label: "🔊 Audio" },
  { id: "bluetooth", label: "🔵 Bluetooth" },
  { id: "modules", label: "🧩 Modules" },
  { id: "system", label: "🖥 System" },
  { id: "power", label: "🔋 Power" },
  { id: "services", label: "🛠 Services" },
];

/** Reconcile a saved panel order with the current PANEL_META: keep known ids in
 *  their saved order, then append any new panels (and drop unknown ones). Keeps
 *  the nav correct when panels are added/removed across versions. */
export function effectivePanelOrder(order: string[]): string[] {
  const all = PANEL_META.map((p) => p.id);
  const kept = order.filter((id) => all.includes(id));
  return [...kept, ...all.filter((id) => !kept.includes(id))];
}

export const DEFAULT_SETTINGS: Settings = {
  theme: "dark",
  accent: "#89b4fa",
  density: "comfortable",
  fontScale: "md",
  remember: true,
  notifications: true,
  panelOrder: ["devices", "audio", "bluetooth", "modules", "system", "power", "services"],
  hiddenPanels: [],
  startupView: "devices",
};

const SETTINGS_KEY = "dmgr.settings";
const UI_KEY = "dmgr.ui";

export function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (raw) return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
  } catch {
    /* ignore corrupt storage */
  }
  return { ...DEFAULT_SETTINGS };
}

export function saveSettings(s: Settings): void {
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(s));
  } catch {
    /* ignore */
  }
}

/** Apply theme/accent/density to the document root. */
export function applySettings(s: Settings): void {
  const root = document.documentElement;
  root.setAttribute("data-theme", s.theme);
  root.setAttribute("data-density", s.density);
  // The app's accent is the `--blue` variable; overriding it recolors primary
  // buttons, active nav, focus rings and sliders in one shot.
  root.style.setProperty("--blue", s.accent);
  root.style.setProperty("--accent", s.accent);
  // Overall interface scale via `zoom` (WebKitGTK supports it) — scales the
  // px-based layout coherently without a rem refactor.
  const fs = FONT_SCALES.find((f) => f.id === s.fontScale) ?? FONT_SCALES[1];
  root.style.setProperty("zoom", String(fs.zoom));
}

export function loadUiState(): Partial<UiState> | null {
  try {
    const raw = localStorage.getItem(UI_KEY);
    if (raw) return JSON.parse(raw);
  } catch {
    /* ignore */
  }
  return null;
}

export function saveUiState(ui: UiState): void {
  try {
    localStorage.setItem(UI_KEY, JSON.stringify(ui));
  } catch {
    /* ignore */
  }
}

export function clearUiState(): void {
  try {
    localStorage.removeItem(UI_KEY);
  } catch {
    /* ignore */
  }
}
