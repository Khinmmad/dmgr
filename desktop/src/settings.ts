// User customization: theme, accent color, density — persisted in localStorage
// and applied via attributes/CSS variables on <html>.

export type Theme = "dark" | "light";
export type Density = "comfortable" | "compact";

export interface Settings {
  theme: Theme;
  accent: string; // hex color
  density: Density;
  remember: boolean; // remember last view & filters across sessions
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

export const DEFAULT_SETTINGS: Settings = {
  theme: "dark",
  accent: "#89b4fa",
  density: "comfortable",
  remember: true,
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
