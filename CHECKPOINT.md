# CHECKPOINT — dmgr-desktop session

> **Read this first** if you are a fresh agent / new session picking up dmgr-desktop work.
> Maintained by: isra (Khinmmad). Last updated: 2026-06-14.

---

## TL;DR

- Local repo has a pending fix on branch **`fix-aur-localhost-and-windows`**: tag **`v2.1.1`** created locally (commit `b1df46a`).
- AUR package **`dmgr-desktop 2.1.0-4`** is installed and working (the "localhost failed" error is fixed).
- The new **`v2.1.1`** adds an inline SVG `GearIcon` so the settings button is visible on systems whose text font lacks the U+2699 `⚙` glyph (e.g. Hyprland).
- **NEXT IMMEDIATE**: user pushes the v2.1.1 tag to GitHub, agent updates the AUR PKGBUILD to `pkgver=2.1.1 pkgrel=1`.
- **THEN**: perfect the **Bluetooth** module (full audit + plan below).

---

## Current state

### Build status
- ✅ `npm run build` (frontend) passes.
- ✅ `cargo build --release --features custom-protocol` passes.
- ✅ AUR package `dmgr-desktop 2.1.0-4` was built and installed via `yay -S dmgr-desktop`. No "localhost failed" at runtime, app launches normally.
- ✅ TypeScript typecheck passes.

### Git state (local repo)
- Branch: `fix-aur-localhost-and-windows`
- Latest commit: `b1df46a fix(desktop): replace missing ⚙ glyph with inline SVG gear icon`
- Tag: `v2.1.1` created **locally only** (not on GitHub yet).
- Working tree: clean.

### AUR state
- Repo: `ssh://aur@aur.archlinux.org/dmgr-desktop.git`
- Last commit on AUR: `cfc809a pkgrel=4: inject custom-protocol feature (v2.1.0 tarball lacks it)`
- Current AUR package: `2.1.0-4` (uses sed patch to add the `[features]` block to the v2.1.0 tarball's `Cargo.toml`).

### Files changed in this session
- `desktop/src/components/GearIcon.tsx` (new) — inline SVG gear.
- `desktop/src/App.tsx` — uses `<GearIcon />` for brand and settings button.
- `desktop/package.json`, `desktop/src-tauri/Cargo.toml`, `desktop/src-tauri/tauri.conf.json` — version `2.1.0 → 2.1.1`.
- `aur-dmgr-desktop/PKGBUILD` (commits `da087f4`, `cfc809a`):
  - `pkgrel=2 → 4`.
  - Added `prepare()` that sed-injects the `[features]` block into the v2.1.0 tarball.
  - Added `--features custom-protocol` to the cargo build line.

---

## Pending immediate steps (in order)

1. **USER** — push the new commit and tag:
   ```bash
   cd /home/isra/projects/dmgr
   git push origin fix-aur-localhost-and-windows
   git push origin v2.1.1
   ```
   (Optionally merge into `main` first and tag there — user's call.)

2. **AGENT** — once the tag is on GitHub, update the AUR PKGBUILD:
   - `pkgver=2.1.1`, `pkgrel=1`.
   - Drop the `prepare()` sed block — the v2.1.1 tarball will already have the `[features]` block in `Cargo.toml` (the local repo has it from a previous commit).
   - Keep `cargo build --release --features custom-protocol` (still needed; Tauri CLI sets it, plain `cargo build` does not).
   - Regenerate `.SRCINFO` and commit + push to the AUR.

3. **USER** — `yay -Syu dmgr-desktop` to get the new build with the visible settings button.

---

## Key technical decisions and why

### Why `custom-protocol` was missing in v2.1.0
The published v2.1.0 tag on GitHub was cut **before** the `custom-protocol` feature was added to `desktop/src-tauri/Cargo.toml`. The local repo has the feature (and even the Windows-only deps), but the v2.1.0 tarball does not. The AUR PKGBUILD was copying the in-repo `packaging/dmgr-desktop/PKGBUILD` which had `--features custom-protocol` set, so `cargo` errored with "the package 'dmgr-desktop' does not contain this feature: custom-protocol".

We worked around it for `2.1.0-4` with a `prepare()` step in the AUR PKGBUILD that sed-injects the `[features]` block. With `v2.1.1`, the tarball will have the feature natively, so the patch becomes unnecessary and can be removed.

### Why the inline SVG GearIcon
The `⚙` character (U+2699 GEAR) is not in many Linux/Wayland text fonts. The user reported it rendering as an invisible Tofu box on Hyprland. Using an inline SVG (Feather "settings" icon) avoids any font dependency and uses `currentColor` for themeability.

Other Unicode characters in the app (🔊 U+1F50A, 🔵 U+1F535, 🧩 U+1F9E9) work because they come from Noto Color Emoji, which is universally installed. Single-glyph rare symbols like `⚙` (U+2699), `⟳` (U+27F3), `★`/`☆` (U+2605/U+2606) may have the same issue and should be replaced with SVG if reported missing. See "Bluetooth frontend" below — the trust button uses `★`/`☆`.

### Tauri 2 + WebKitGTK blank window on Nvidia + Wayland
Worked around in `src-tauri/src/platform.rs` — sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` automatically. Override by exporting your own value before launching.

---

## Bluetooth module — perfection plan

### Current state
- **Linux** (`src-tauri/src/bluetooth.rs` → `unix_impl`): 1× `bluetoothctl show` + 1× `bluetoothctl devices` + **N× `bluetoothctl info`** per state poll. Frontend polls every 5 s. No async, no timeout, no incremental updates, no events.
- **Windows** (`win_impl`): PnP-based read-only listing. Connect/disconnect/trust return explicit "not supported" errors. Power toggle goes through PnP enable/disable of the radio (heavy-handed, disconnects all devices).
- **No macOS** support.
- Linux: zero unit tests. Windows: 5 tests (`mac_from_dev_form`, `mac_from_transport_form_ignores_base_uuid`, `mac_from_ble_form_ignores_base_uuid`, `no_mac_returns_none`, `subprofile_detection`, `icon_heuristics`).

### Audit — what's "imperfect" right now

#### Performance
- [P0] Per-device `bluetoothctl info` spawns N subprocesses. For 10 paired devices = 12 spawns / poll. Fix: batch via `bluetoothctl --json` (BlueZ 5.66+) or single multi-device call.
- [P1] Sync subprocess calls block the Tauri runtime. Fix: `tokio::process::Command`.
- [P1] No debounce on rapid user clicks. Fix: in-flight guard per `(mac, action)`.
- [P2] 5 s polling is wasteful. Fix: `bluetoothctl monitor` (D-Bus signal under the hood) for push events, exposed as a Tauri event.

#### Robustness
- [P0] No timeout on `bluetoothctl` calls. Fix: `tokio::time::timeout` (3 s default).
- [P1] String parsing is fragile (relies on exact `bluetoothctl` output). Fix: prefer `--json` or DBus.
- [P1] `available: true` when `bluetoothctl` is installed but `bluetoothd` is not. Fix: also check `bluetoothctl list`.
- [P2] Errors are plain strings. Fix: typed error enum (NotFound, NotPowered, Failed, Timeout, NoAdapter).

#### Features
- [P1] No **scan / discover** — can't pair a new device from the UI. Fix: `bluetoothctl scan on` (timeout 10–15 s) + new command.
- [P1] No **unpair** — can't unpair a device. Fix: `bluetoothctl remove <mac>` + new command.
- [P2] No device details (battery, type, signal) — `bluetoothctl info` exposes more. Fix: extend `BtDevice` and surface in a details modal.
- [P2] No **macOS** support. Out of scope unless requested.

#### Frontend (`BluetoothPanel.tsx`)
- [P1] 5 s polling is wasteful. Fix: event-driven via `listen("bluetooth-changed", …)`.
- [P1] Trust button uses `★`/`☆` (U+2605/U+2606) — same font risk as `⚙`. Fix: inline SVG stars (only if user reports missing) or text labels "Trust" / "Untrust".
- [P2] No loading skeleton on first render.
- [P2] "No paired devices" is conflated with "no adapter".
- [P2] No device details modal.

#### Testing
- [P1] No tests on Linux impl. Fix: golden-output fixtures (recorded `bluetoothctl` output) parsed by a pure function we extract.

### Proposed order of work
1. **Robustness** (foundation) — timeouts, typed errors, async, no-op when bluetoothd is down.
2. **Performance** — batched info call, debounce, event-driven updates.
3. **Features** — scan, unpair, details (battery).
4. **Frontend** — event-driven updates, empty-state distinction, optional SVG icons.
5. **Tests** — Linux side, golden fixtures.

### Key files
- Backend: `desktop/src-tauri/src/bluetooth.rs` (420 lines)
- Frontend: `desktop/src/components/BluetoothPanel.tsx` (174 lines)
- Tauri commands: `desktop/src-tauri/src/commands.rs`
- TS API: `desktop/src/api.ts` (look for `btState`, `btSetPower`, `btSetTrust`, `btConnect`, `btDisconnect`, `openBluetoothSettings`)
- Types: `desktop/src/types.ts` (look for `BtState`, `BtDevice`)

---

## Useful files to read on a fresh session

- `AGENTS.md` — project conventions, build/test commands, known caveats.
- `desktop/README.md` — Tauri build notes (the `--features custom-protocol` gotcha).
- `desktop/src-tauri/Cargo.toml` — features block.
- `aur-dmgr-desktop/PKGBUILD` — the AUR one we maintain.
- `packaging/dmgr-desktop/PKGBUILD` — in-repo reference (the source of truth).
- `desktop/src-tauri/src/bluetooth.rs` — start here for BT work.

## Conventions (from AGENTS.md)
- One commit per fix, descriptive messages.
- Update `AGENTS.md` and this `CHECKPOINT.md` if project structure / patterns change.
- The old `PROGRESS.md` reference is stale — the user uses `AGENTS.md` as the single source of truth.
