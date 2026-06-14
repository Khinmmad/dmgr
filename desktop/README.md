# dmgr-desktop — Tauri + React frontend

A modern device-manager UI for `dmgr`, built on **Tauri v2** (Rust backend) and
**React + TypeScript** (Vite). It reuses `crates/dmgr-core` for all device logic
and the existing `dmgr-polkit-helper` for privileged operations. The legacy
`dmgr-gui` (egui) crate is left untouched as a fallback.

## Features

- **Smart sidebar** — devices grouped by bus; irrelevant/dead devices are hidden
  by default (toggle **Show all**), keeping active and actionable devices in view.
- **Device actions (Windows Device Manager style)** — enable/disable (`authorized`
  flag), install/change driver (bind), uninstall driver (unbind), live rescan.
- **Properties** — view every sysfs property; edit the ones the kernel allows.
- **Dedicated Audio panel** — all connected output devices, the active one
  highlighted in green, one-click switching, per-device volume and mute. Inputs too.
- **Bluetooth panel** — connect/disconnect/trust paired devices, toggle the adapter.

## Develop

```bash
cd desktop
npm install
npm run tauri dev      # hot-reload dev window
```

## Build a release binary

```bash
cd desktop
npm run build                       # frontend -> dist/
cargo build --release --features custom-protocol --manifest-path src-tauri/Cargo.toml
./src-tauri/target/release/dmgr-desktop
```

The frontend is embedded in the binary, so it runs standalone.

> **`--features custom-protocol` is mandatory.** Without it, the `tauri` crate
> compiles in dev mode and the binary tries to load the Vite dev server at
> `http://localhost:1420` at runtime ("localhost failed"). `cargo tauri build`
> enables this automatically; plain `cargo build` does not. Equivalently you can
> run `npm run tauri build -- --no-bundle` to get just the binary.

## Packaging

**Arch (AUR) — full install (recommended):**
```bash
cd packaging/dmgr-desktop && makepkg -si
```
Installs the GUI binary, `dmgr-polkit-helper`, the polkit policy, `.desktop` and icons.

**.deb / .rpm (other distros):**
```bash
# Build the privileged helper first — the bundles include it.
cargo build --release -p dmgr-polkit-helper
cd desktop
npm run tauri build -- --bundles deb,rpm
# → src-tauri/target/release/bundle/{deb,rpm}/
```
Tauri's deb/rpm bundlers are pure-Rust (no `dpkg-deb`/`rpmbuild` required). The
bundles ship the GUI **plus** `dmgr-polkit-helper` and the polkit policy (via
`bundle.linux.{deb,rpm}.files`), so privileged actions work out of the box.

> Note: building the helper first is required — `tauri build` references
> `target/release/dmgr-polkit-helper`. The AUR package handles this automatically.

**Windows:**
```powershell
cd desktop
npm install
npm run build
# Standalone binary (note custom-protocol, same as Linux):
cargo build --release --features custom-protocol --manifest-path src-tauri/Cargo.toml
# Or the NSIS installer (the CLI sets custom-protocol for you):
npm run tauri build -- --bundles nsis
```
The Windows backend enumerates devices natively (SetupAPI, with a
`Get-PnpDevice` fallback), supports Enable/Disable via UAC self-elevation,
Core Audio (WASAPI) output/input switching with volume & mute, Bluetooth
listing + adapter toggle, and live hotplug refresh (`CM_Register_Notification`).
Kernel modules remain Linux-only. The installer (`…x64-setup.exe`) installs
per-user — no Administrator needed.

### Windows code signing & SmartScreen

The published installer is **not code-signed**, so Windows SmartScreen shows
an "unknown publisher" warning on first run (users can continue via
*More info → Run anyway*). Defender's antivirus itself reports the binaries
clean. To remove the warning:

- **[SignPath Foundation](https://signpath.org/about)** — free code-signing for
  qualifying open-source projects (CI-integrated). Best $0 option.
- **Azure Trusted Signing** — Microsoft's signing service (~$10/month).
- A classic **OV code-signing certificate** (~$100–400/year). Note SmartScreen
  reputation still builds over time even when signed.
- You can also report the file as a false positive / request reputation review:
  <https://www.microsoft.com/en-us/wdsi/filesubmission>.

`SHA256SUMS.txt` is published with each release so users can verify downloads:
`Get-FileHash dmgr-desktop_2.1.0_x64-setup.exe -Algorithm SHA256`.

## Notes

- On **Nvidia + Wayland**, WebKitGTK is started with
  `WEBKIT_DISABLE_DMABUF_RENDERER=1` automatically (set inside `run()`), which
  avoids a blank window. Override by exporting your own value.
- Privileged actions (bind/unbind/set property/enable-disable) are routed through
  `pkexec dmgr-polkit-helper`. Build the helper (`cargo build` at repo root) or
  install dmgr so the helper is on `PATH`.
- This is a **nested, independent Cargo workspace** (`src-tauri/Cargo.toml` declares
  its own `[workspace]`), so the repo-root workspace and its `cargo test` are
  unaffected.
