# dmgr-desktop — Roadmap

Planned improvements for the **Tauri + React** desktop frontend (`desktop/`), agreed 2026-06-07.
Listed in dependency order — do the sprints top to bottom. The backend abstraction (Sprint 1a)
unblocks Windows, multi-backend audio and multi-distro support, so it comes first.

> Context: `desktop/` is a nested independent Cargo workspace. Backend = `desktop/src-tauri/src/`
> (reuses `crates/dmgr-core`). Frontend = `desktop/src/` (React + TS + Vite). Legacy `crates/dmgr-gui`
> (egui) is kept as a fallback and must not break. Root workspace `cargo test` must keep passing.

---

## Sprint 1 — Foundation + quick wins ✅ DONE (2026-06-07)

Delivered in `desktop/src-tauri/src/backend/` (trait + `linux.rs`), `hotplug.rs`,
and `desktop/src/components/DeviceTree.tsx`. Backend trait now backs all device
commands via Tauri managed state; live udev hotplug emits `devices-changed`;
sidebar has a Bus/Tree toggle. Verified: builds clean, app launches, "hotplug
monitor started" confirmed, root `cargo test` unaffected.

### 1a. `DeviceBackend` trait abstraction — ✅
- **Goal:** route every Tauri device command through a `Box<dyn DeviceBackend>` chosen at runtime by OS,
  so a `WindowsBackend` can be added later without touching the UI or command layer.
- **Where:** new module e.g. `crates/dmgr-core/src/backend.rs` (trait + `LinuxBackend`) or a thin layer in
  `desktop/src-tauri/src/`. Keep `dmgr-core` free functions intact (egui + tests depend on them) — the
  trait should *delegate* to them, not replace them.
- **Trait sketch:**
  ```rust
  pub trait DeviceBackend: Send + Sync {
      fn scan(&self) -> Result<Vec<Device>>;
      fn available_drivers(&self, path: &str) -> Result<Vec<String>>;
      fn get_property(&self, path: &str, prop: &str) -> Result<Option<String>>;
      fn set_property(&self, path: &str, prop: &str, val: &str) -> Result<()>;
      fn bind(&self, path: &str, driver: &str) -> Result<()>;
      fn unbind(&self, path: &str) -> Result<()>;
      fn set_enabled(&self, path: &str, enabled: bool) -> Result<()>;
  }
  ```
- **Acceptance:** `commands.rs` calls the trait; Linux behaviour identical; `cargo build` + root `cargo test` green.

### 1b. Live hotplug (udev events → UI) — ✅
- **Goal:** UI refreshes itself on connect/disconnect; highlight newly-added devices (fade green, like egui v2 did).
- **Where:** `crates/dmgr-core/src/udev.rs` already has `UdevMonitor` (mpsc channel). In `lib.rs`, spawn a thread
  that forwards `UdevEvent`s as Tauri events via `app.emit("device-changed", ...)`. Frontend: `listen()` in
  `App.tsx`, debounce, re-scan.
- **Acceptance:** plugging a USB device updates the sidebar within ~1s without manual refresh.

### 1c. Hierarchical tree view — ✅
- **Goal:** parent/child tree like Windows Device Manager (USB hubs → devices, PCI bridges → functions).
- **Where:** `Device.parent` / `Device.children` already populated. Build the tree in `Sidebar.tsx` (or a new
  `DeviceTree.tsx`); add expand/collapse. Offer a toggle between "by bus" (current) and "by hierarchy".
- **Acceptance:** USB hubs show their children nested; collapsing works; relevance filter still applies.

---

## Sprint 2 — Universal Linux compatibility ✅ DONE (2026-06-07)

- **Audio backends — ✅** `desktop/src-tauri/src/audio/` is now a module with an `AudioBackend` trait and
  `pactl` / `wpctl` (WirePlumber) / `alsa` (read-only) impls; `detect()` picks the first available (cached).
  `commands.rs` routes through it; `capabilities`/`platform_info` report the active backend.
- **Permissions without polkit — ✅** `privileged.rs` gained `can_elevate()` and pkexec detection with clear,
  actionable error messages; the UI shows a "⚠ no root" status chip with a distro-aware install hint.
- **Multi-distro / no systemd — ✅** `platform.rs` reads `/etc/os-release` (id + pretty name) and maps a
  `package_hint` per distro family (pacman/apt/dnf/zypper/xbps/apk/emerge/nix). Surfaced in the status bar.
- **X11 + Wayland / multi-GPU — ✅** the WebKit DMABUF flag now applies **only** on Nvidia + Wayland
  (`platform::apply_webkit_workarounds`), leaving Intel/AMD and X11 on the faster default path.

## Sprint 3 — Depth ✅ DONE (2026-06-07)

- **Advanced details panel — ✅** `details.rs` reads a curated set of sysfs attributes per bus (PCIe link
  speed/width, IRQ, MSI count; USB version/speed/power; Net MAC/speed/MTU; Block size/scheduler; DRM status;
  plus runtime-PM for all). No lspci/lsusb dependency. Collapsible, lazy-loaded "Advanced details" section in
  `DeviceDetail.tsx` via `advanced_details` command.
- **Kernel module management — ✅** `kernel.rs` parses `/proc/modules`, `modinfo` for details, and load/unload via
  `modprobe` through `privileged::run_pkexec` (new generic pkexec runner). New **Modules** tab
  (`ModulesPanel.tsx`): search, expand for modinfo, load by name, unload (with in-use confirm).

## Sprint 4 — Linux packaging ✅ DONE (2026-06-07)

- **AUR — ✅** `packaging/dmgr-desktop/PKGBUILD` builds the frontend + Tauri backend + the polkit helper, and
  installs the binary, `dmgr-polkit-helper`, polkit policy, `.desktop` and icons. Full-featured install.
- **.deb / .rpm — ✅** `tauri.conf.json` bundle is configured (category, descriptions, deb/rpm depends).
  `npm run tauri build -- --bundles deb,rpm` produces both (Tauri's pure-Rust bundlers — no dpkg/rpmbuild
  needed). Verified: `dmgr-desktop_2.0.0_amd64.deb` + `dmgr-desktop-2.0.0-1.x86_64.rpm` build and contain the
  binary, icon and desktop entry. Also `resources/dmgr-desktop.desktop` added.
- **Known limitation / follow-up:** the deb/rpm bundle ships only the GUI binary — the privileged helper and
  polkit policy are NOT included, so root actions need `dmgr-polkit-helper` installed separately (the AUR
  package includes it). To make deb/rpm self-sufficient, pre-build the helper and add it via
  `bundle.linux.{deb,rpm}.files`.

## Sprint 5 — Windows ⚠ WRITTEN, UNVERIFIED (2026-06-07)

- **`WindowsBackend` — written (untested)** `desktop/src-tauri/src/backend/windows.rs` implements the
  `DeviceBackend` trait via **PowerShell** (`Get-PnpDevice` → JSON → `Device`, `Get-PnpDeviceProperty`,
  `Enable-PnpDevice`/`Disable-PnpDevice` for enable/disable). `#[cfg(target_os = "windows")]`, wired in
  `backend/mod.rs::current_backend()`. **Cannot be compiled/run from the Arch dev box** (no Windows target or
  mingw installed), so it's unverified — needs a Windows machine or CI to validate.
- **MSI installer:** on Windows, `npm run tauri build -- --bundles nsis` (or `msi`) produces the installer;
  Tauri handles NSIS/WiX. The Linux `bundle.targets` is left as deb/rpm/appimage so Linux builds don't break.
- **Windows follow-ups:** audio/Bluetooth/kernel-module panels are Linux-only and degrade to empty on Windows;
  driver enumeration (pnputil) and parent/child hierarchy aren't mapped yet. Consider switching from PowerShell
  to the `windows` crate (SetupAPI/CfgMgr32) for speed once it can be tested.

---

## Decisions captured
- Windows strategy: **abstract now, implement later** (Sprint 1a prepares it; Sprint 5 implements).
- Wanted features: all four (hotplug, tree, kernel modules, advanced details).
- Compat: all four (audio backends, multi-distro, X11+Wayland, no-polkit fallback).
- Packaging: AUR + .deb/.rpm + Windows MSI.
