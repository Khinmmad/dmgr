// Thin typed wrappers around the Tauri command layer.
import { invoke } from "@tauri-apps/api/core";
import type {
  AudioApp,
  AudioDevice,
  BtDevice,
  BtState,
  Capabilities,
  DetailItem,
  Device,
  KernelModule,
  ModuleInfo,
  Platform,
  PowerInfo,
  SystemInfo,
} from "./types";

export const api = {
  // devices
  scanDevices: () => invoke<Device[]>("scan_devices"),
  availableDrivers: (path: string) =>
    invoke<string[]>("get_available_drivers", { path }),
  getProperty: (path: string, property: string) =>
    invoke<string | null>("get_property", { path, property }),
  setProperty: (path: string, property: string, value: string) =>
    invoke<void>("set_property", { path, property, value }),
  bindDriver: (path: string, driver: string) =>
    invoke<void>("bind_driver", { path, driver }),
  unbindDriver: (path: string) => invoke<void>("unbind_driver", { path }),
  reloadDriver: (driver: string) => invoke<void>("reload_driver", { driver }),
  setDeviceEnabled: (path: string, enabled: boolean) =>
    invoke<void>("set_device_enabled", { path, enabled }),

  // audio
  audioOutputs: () => invoke<AudioDevice[]>("audio_outputs"),
  audioInputs: () => invoke<AudioDevice[]>("audio_inputs"),
  setDefaultOutput: (name: string) =>
    invoke<void>("audio_set_default_output", { name }),
  setDefaultInput: (name: string) =>
    invoke<void>("audio_set_default_input", { name }),
  setVolume: (name: string, percent: number) =>
    invoke<void>("audio_set_volume", { name, percent }),
  setMute: (name: string, muted: boolean) =>
    invoke<void>("audio_set_mute", { name, muted }),
  audioAppStreams: () => invoke<AudioApp[]>("audio_app_streams"),
  setAppVolume: (index: number, percent: number) =>
    invoke<void>("audio_set_app_volume", { index, percent }),
  setAppMute: (index: number, muted: boolean) =>
    invoke<void>("audio_set_app_mute", { index, muted }),

  // bluetooth
  btState: () => invoke<BtState>("bt_state"),
  btPair: (mac: string) => invoke<void>("bt_pair", { mac }),
  btConnect: (mac: string) => invoke<void>("bt_connect", { mac }),
  btDisconnect: (mac: string) => invoke<void>("bt_disconnect", { mac }),
  btSetPower: (on: boolean) => invoke<void>("bt_set_power", { on }),
  btSetTrust: (mac: string, trust: boolean) =>
    invoke<void>("bt_set_trust", { mac, trust }),
  btRemove: (mac: string) => invoke<void>("bt_remove", { mac }),
  btScan: (secs?: number) =>
    invoke<BtDevice[]>("bt_scan", { secs: secs ?? null }),

  // advanced details
  advancedDetails: (path: string, bus: string) =>
    invoke<DetailItem[]>("advanced_details", { path, bus }),

  // kernel modules
  kernelModules: () => invoke<KernelModule[]>("kernel_modules"),
  kernelModuleInfo: (name: string) =>
    invoke<ModuleInfo>("kernel_module_info", { name }),
  kernelModuleLoad: (name: string) =>
    invoke<void>("kernel_module_load", { name }),
  kernelModuleUnload: (name: string) =>
    invoke<void>("kernel_module_unload", { name }),

  // meta
  capabilities: () => invoke<Capabilities>("capabilities"),
  platformInfo: () => invoke<Platform>("platform_info"),
  systemInfo: () => invoke<SystemInfo>("system_info"),
  powerInfo: () => invoke<PowerInfo>("power_info"),
  setPowerProfile: (profile: string) => invoke<void>("power_set_profile", { profile }),
  setBrightness: (percent: number) => invoke<void>("power_set_brightness", { percent }),

  // windows shortcuts
  openDeviceManager: () => invoke<void>("open_device_manager"),
  openBluetoothSettings: () => invoke<void>("open_bluetooth_settings"),
};
