// Mirrors the JSON produced by dmgr-core (serde) and the Tauri command layer.

export type Bus =
  | "Usb"
  | "Pci"
  | "Audio"
  | "Input"
  | "Block"
  | "Drm"
  | "Net"
  | "Hid"
  | "Tty"
  | "Power"
  | "IoMMU"
  | "Unknown";

export type DeviceStatus =
  | "Online"
  | "Offline"
  | "Suspended"
  | "Unbound"
  | "Error"
  | "Unknown";

export interface Interface {
  name: string;
  class: string;
  protocol: string | null;
}

export interface Device {
  id: string;
  name: string;
  bus: Bus;
  bus_id: string | null;
  vendor: string | null;
  vendor_id: string | null;
  model: string | null;
  model_id: string | null;
  driver: string | null;
  status: DeviceStatus;
  subsystem: string;
  path: string;
  parent: string | null;
  children: string[];
  interfaces: Interface[];
  properties: Record<string, string>;
  editable_properties: string[];
  removable: boolean;
  authorized: boolean;
}

export interface AudioDevice {
  index: number;
  name: string;
  description: string;
  state: string; // RUNNING | SUSPENDED | IDLE
  muted: boolean;
  volume: number | null;
  is_default: boolean;
  kind: "Builtin" | "Usb" | "Hdmi" | "Bluetooth" | "Virtual";
}

export interface BtDevice {
  mac: string;
  name: string;
  paired: boolean;
  connected: boolean;
  trusted: boolean;
  icon: string;
  battery: number | null;
  rssi: number | null;
}

export interface BtState {
  available: boolean;
  powered: boolean;
  devices: BtDevice[];
}

export interface Capabilities {
  audio: boolean;
  audio_backend: string; // pactl | wpctl | alsa | none
  bluetooth: boolean;
  root: boolean;
}

export interface DetailItem {
  label: string;
  value: string;
}

export interface KernelModule {
  name: string;
  size_kb: number;
  refcount: number;
  used_by: string[];
  state: string;
}

export interface ModuleInfo {
  name: string;
  filename: string | null;
  description: string | null;
  author: string | null;
  license: string | null;
  version: string | null;
  depends: string[];
  params: string[];
}

export interface Platform {
  os: string;
  distro_id: string;
  distro_name: string;
  session: string; // wayland | x11 | unknown
  gpu_nvidia: boolean;
  audio_backend: string;
  can_elevate: boolean;
  package_hint: string;
}

// ── UI helpers ────────────────────────────────────────────────────────────────

export const BUS_META: Record<Bus, { label: string; icon: string }> = {
  Usb: { label: "USB", icon: "🔌" },
  Pci: { label: "PCIe", icon: "🖥" },
  Audio: { label: "Audio", icon: "🔊" },
  Input: { label: "Input", icon: "⌨" },
  Block: { label: "Storage", icon: "💾" },
  Drm: { label: "GPU / Display", icon: "🎮" },
  Net: { label: "Network", icon: "🌐" },
  Hid: { label: "HID", icon: "🖱" },
  Tty: { label: "Serial / TTY", icon: "🔣" },
  Power: { label: "Power", icon: "🔋" },
  IoMMU: { label: "IOMMU", icon: "🧩" },
  Unknown: { label: "Other", icon: "❓" },
};

export const STATUS_META: Record<DeviceStatus, { label: string; color: string; dot: string }> = {
  Online: { label: "Active", color: "#4ade80", dot: "🟢" },
  Suspended: { label: "Suspended", color: "#facc15", dot: "🟡" },
  Offline: { label: "Offline", color: "#f87171", dot: "🔴" },
  Unbound: { label: "No driver", color: "#cbd5e1", dot: "⚪" },
  Error: { label: "Error", color: "#fb923c", dot: "🟠" },
  Unknown: { label: "Unknown", color: "#64748b", dot: "⚫" },
};

// Buses considered "noise" — hidden unless "Show all" is on.
export const NOISE_BUSES: Bus[] = ["Tty", "IoMMU", "Unknown", "Power"];
