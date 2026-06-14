# CHECKPOINT — dmgr-desktop session

> **Read this first** if you are a fresh agent / new session picking up dmgr-desktop work.
> Maintained by: isra (Khinmmad). Last updated: 2026-06-14 (post-Bluetooth work).

---

## TL;DR

- AUR package **`dmgr-desktop 2.1.1-1`** is live (user pushed v2.1.1, agent updated AUR: `pkgver=2.1.1 pkgrel=1`, dropped the sed patch).
- Local branch **`fix-aur-localhost-and-windows`** has 4 commits past v2.1.1 — GearIcon fix + Bluetooth perfection (async, timeouts, typed errors, scan, unpair, in-flight guards, daemon-down banner).
- **NOT YET SHIPPED** to GitHub/AUR: those 2 BT commits. User runs `yay -S dmgr-desktop` to get 2.1.1-1 (settings button visible). The BT work is in the local branch and ready to be tagged as v2.1.2.
- **NEXT**: user cuts v2.1.2 (or merges branch + tags), agent updates AUR to `2.1.2-1`.

---

## Current state

### Build status
- ✅ `npm run build` passes.
- ✅ `cargo build --release --features custom-protocol` passes.
- ✅ AUR `2.1.1-1` builds and installs (verified locally).
- ✅ 15 lib tests pass (6 new BT Linux tests + 9 existing).
- ✅ Frontend typecheck + vite build clean.

### Git state (local repo)
- Branch: `fix-aur-localhost-and-windows`
- Commits past v2.1.1:
  - `b1df46a` — GearIcon fix (now in v2.1.1)
  - `095345b` — CHECKPOINT.md + AGENTS.md link
  - `432bfd5` — BT Phase 1: async + timeouts + typed errors + Linux tests
  - `fdaeeb2` — BT Phase 2+3: scan, unpair, daemon banner, in-flight guards
- Working tree: clean.

### AUR state
- Repo: `ssh://aur@aur.archlinux.org/dmgr-desktop.git`
- Last AUR commit: `1012ddc v2.1.1-1: drop custom-protocol sed patch (now in upstream tarball)`
- Current AUR: `2.1.1-1` (GearIcon fix only — the BT work is in the local branch but not yet tagged).

### Files changed in this session (cumulative)
- `desktop/src/components/GearIcon.tsx` (new).
- `desktop/src/App.tsx` — `<GearIcon />` for brand and settings button.
- `desktop/package.json`, `desktop/src-tauri/Cargo.toml`, `desktop/src-tauri/tauri.conf.json` — `2.1.0 → 2.1.1`.
- `aur-dmgr-desktop/PKGBUILD` — `pkgver=2.1.1 pkgrel=1`, sed `prepare()` block dropped.
- `desktop/src-tauri/Cargo.toml` — `+ thiserror = "1"`, `+ tokio = { ..., features = ["process", "time", "macros", "rt"] }`.
- `desktop/src-tauri/src/bluetooth.rs` — full refactor: `BtError` enum, async fn, timeouts, daemon-down detection, parallel `info()` calls, 6 new unit tests, `scan()` + `remove()` (Linux), typed `BtError` stubs for Windows.
- `desktop/src-tauri/src/commands.rs` — `bt_state/bt_connect/bt_disconnect/bt_set_power/bt_set_trust/bt_remove/bt_scan/capabilities` now `async fn`.
- `desktop/src-tauri/src/lib.rs` — registered `commands::bt_remove` and `commands::bt_scan`.
- `desktop/src/api.ts` — `btRemove(mac)`, `btScan(secs?)`.
- `desktop/src/components/BluetoothPanel.tsx` — per-action in-flight guards, Scan button (Linux), Unpair button, daemon-down banner, text-based Trust labels (replaced `★`/`☆`).
- `CHECKPOINT.md` — this file.
- `AGENTS.md` — one-line pointer to `CHECKPOINT.md`.

---

## Pending immediate steps (in order)

1. **USER** — tag and push the BT work as `v2.1.2`:
   ```bash
   cd /home/isra/projects/dmgr
   git checkout main && git merge --no-ff fix-aur-localhost-and-windows   # optional
   git tag -a v2.1.2 -m "dmgr-desktop 2.1.2 (bluetooth perfection)"
   git push origin main fix-aur-localhost-and-windows v2.1.2
   ```
   (Skip the merge if you prefer to keep the fix branch separate.)

2. **AGENT** — once `v2.1.2` is on GitHub, update the AUR PKGBUILD:
   - `pkgver=2.1.2`, `pkgrel=1`.
   - Bump versions in the same three files (`package.json`, `Cargo.toml`, `tauri.conf.json`) — but actually, the version in those files is the *product* version, not the package. The AUR's `pkgver` follows the product version. So as long as the new tag is `v2.1.2`, the source URL auto-updates to `v2.1.2.tar.gz`.
   - Regenerate `.SRCINFO` and commit + push to the AUR.

3. **USER** — `yay -Syu dmgr-desktop` to get the BT improvements.

---

## Key technical decisions and why

### Why `custom-protocol` was missing in v2.1.0
The published v2.1.0 tag on GitHub was cut **before** the `custom-protocol` feature was added to `desktop/src-tauri/Cargo.toml`. The local repo has the feature (and even the Windows-only deps), but the v2.1.0 tarball does not. The AUR PKGBUILD was copying the in-repo `packaging/dmgr-desktop/PKGBUILD` which had `--features custom-protocol` set, so `cargo` errored with "the package 'dmgr-desktop' does not contain this feature: custom-protocol".

We worked around it for `2.1.0-4` with a `prepare()` step in the AUR PKGBUILD that sed-injects the `[features]` block. With `v2.1.1`, the tarball has the feature natively, so the patch was removed (`1012ddc`).

### Why the inline SVG GearIcon
The `⚙` character (U+2699 GEAR) is not in many Linux/Wayland text fonts. The user reported it rendering as an invisible Tofu box on Hyprland. Using an inline SVG (Feather "settings" icon) avoids any font dependency and uses `currentColor` for themeability.

Other Unicode characters in the app (🔊 U+1F50A, 🔵 U+1F535, 🧩 U+1F9E9) work because they come from Noto Color Emoji, which is universally installed. Single-glyph rare symbols like `⚙` (U+2699), `⟳` (U+27F3), `★`/`☆` (U+2605/U+2606) may have the same issue. The trust button's `★`/`☆` were preemptively replaced with text labels "Trust" / "Trusted" in commit `fdaeeb2`. `⟳` (rescan) is still a glyph — leave it until reported missing.

### Bluetooth refactor architecture
- `BtError` enum (`thiserror`) replaces `Result<(), String>`. Serializes to its `Display` string so the TS frontend contract is unchanged.
- All Linux `bluetoothctl` calls are `async` via `tokio::process::Command`, bounded by `QUERY_TIMEOUT` (3s) or `ACTION_TIMEOUT` (8s) via `tokio::time::timeout`.
- Per-device `bluetoothctl info` calls run in parallel (`tokio::spawn`), capped by the timeout. Was O(N+2) sequential spawns per poll; now O(1) wall time (capped by timeout).
- `parse_show` / `parse_info` extracted as pure functions, exercised by 6 new unit tests (parsing + daemon-down detection). Brings total BT tests to 11 (6 Linux + 5 Windows).
- Same async+timeout treatment for the Windows PowerShell path.
- Tauri commands `bt_state/bt_connect/bt_disconnect/bt_set_power/bt_set_trust/bt_remove/bt_scan/capabilities` are now `async fn` (Tauri 2 supports this out of the box).

### Tauri 2 + WebKitGTK blank window on Nvidia + Wayland
Worked around in `src-tauri/src/platform.rs` — sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` automatically. Override by exporting your own value before launching.

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

### Current state (post-`fdaeeb2`)
- **Linux** (`src-tauri/src/bluetooth.rs` → `unix_impl`): async + tokio + timeouts. `bluetoothctl show` + `bluetoothctl devices` + N× `bluetoothctl info` (now parallel). 6 unit tests for parsing.
- **Windows** (`win_impl`): async + tokio + timeouts, PowerShell path. 5 existing tests.
- **No macOS** support (out of scope).
- 11 unit tests total in bluetooth module.

### Done in this session
- [x] **[P0] Timeouts** — 3 s for queries, 8 s for actions (commit `432bfd5`).
- [x] **[P0] Typed errors** — `BtError` enum (`thiserror`); serializes to its `Display` string (commit `432bfd5`).
- [x] **[P0] Async / tokio** — every subprocess call is `async` (commit `432bfd5`).
- [x] **[P1] Daemon-down detection** — `parse_show` returns `(false, false)` when stdout is empty or contains "not available" (commit `432bfd5`).
- [x] **[P1] Parallel info calls** — `tokio::spawn` per device, capped by `QUERY_TIMEOUT` (commit `432bfd5`).
- [x] **[P1] Linux unit tests** — 6 new tests on `parse_show` + `parse_info` (commit `432bfd5`).
- [x] **[P1] In-flight guards** — `inFlight: Set<string>` keyed by action+mac (commit `fdaeeb2`).
- [x] **[P1] Unpair** — `bt_remove` + Unpair button (commit `fdaeeb2`).
- [x] **[P1] Scan** — `bt_scan(secs?)` + Scan button (commit `fdaeeb2`).
- [x] **[P1] Trust glyphs** — replaced `★`/`☆` with text "Trust" / "Trusted" (commit `fdaeeb2`).
- [x] **[P2] Daemon-down banner** — card with `systemctl enable --now bluetooth` hint (commit `fdaeeb2`).
- [x] **[P2] Empty-state distinction** — "no adapter" vs "no paired devices" vs "daemon down" (commit `fdaeeb2`).

### Still pending (for future sessions)
- [P2] **Event-driven updates** — replace 5 s polling with `bluetoothctl monitor` → Tauri event. Medium effort, needs a background task in the backend and `listen("bluetooth-changed", …)` in the panel.
- [P2] **Device details modal** — show battery / type / signal from `bluetoothctl info` (battery requires a separate `bluetoothctl info` field check).
- [P2] **Pair command** — `bluetoothctl pair <mac>`. UX: button visible only when the panel is showing a discovered (unpaired) device. Requires the "discovered devices" section of the UI.
- [P2] **macOS support** — `IOBluetooth` via the `macos-bluetooth` crate or direct FFI. Out of scope unless the user requests it.
- [P3] **Loading skeleton** on first render.
- [P3] **batched info** via `bluetoothctl --json` (BlueZ 5.66+).

### Key files
- Backend: `desktop/src-tauri/src/bluetooth.rs` (~480 lines now)
- Frontend: `desktop/src/components/BluetoothPanel.tsx` (~220 lines now)
- Tauri commands: `desktop/src-tauri/src/commands.rs`
- TS API: `desktop/src/api.ts` (look for `bt*`)
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
