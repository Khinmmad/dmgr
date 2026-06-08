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
cargo build --release --manifest-path src-tauri/Cargo.toml
./src-tauri/target/release/dmgr-desktop
```

The frontend is embedded in the binary, so it runs standalone.

## Packaging

**Arch (AUR) — full install (recommended):**
```bash
cd packaging/dmgr-desktop && makepkg -si
```
Installs the GUI binary, `dmgr-polkit-helper`, the polkit policy, `.desktop` and icons.

**.deb / .rpm (other distros):**
```bash
cd desktop
npm run tauri build -- --bundles deb,rpm
# → src-tauri/target/release/bundle/{deb,rpm}/
```
Tauri's deb/rpm bundlers are pure-Rust (no `dpkg-deb`/`rpmbuild` required).

> ⚠ The deb/rpm bundles ship only the GUI binary. Privileged actions need
> `dmgr-polkit-helper` + the polkit policy installed separately — use the AUR
> package for the complete experience, or build the helper
> (`cargo build --release -p dmgr-polkit-helper`) and install it to `/usr/bin`.

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
