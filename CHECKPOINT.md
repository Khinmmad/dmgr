# CHECKPOINT — dmgr-desktop

> **You are a fresh agent / new session.** Read this top-to-bottom before touching anything. The TL;DR at the top is the only section you need to start a session; everything below is reference.

Maintained by: **isra (Khinmmad)**. Last updated: **2026-06-15, v2.2.0 shipped & verified**.

---

## 🔖 Session handoff — where we stopped

**v2.2.0 SHIPPED & verified this session.** 🎉 The `more-features` batch (13 features
below) merged to `main`, tagged **`v2.2.0`** (merge `816c856`), pushed to GitHub; AUR at
**`2.2.0-1`** (`261658a`). Nothing pending for the release. *(The installed package on
this box is still `2.1.3-1` until the user runs `yay -Syu`/`pacman -U`.)*

**`more-features` batch 1 — UI/UX & BT/audio:**
- `eb21d96` themes (Nord/Gruvbox/Dracula/Macchiato) + interface size (zoom)
- `05feaa4` BT details modal (battery/RSSI/type + auto-connect)
- `08f14c4` device aliases — BT + audio (`useAliases` store, localStorage)
- `0d2fb7a` per-panel search filter + ⭐ favorites (`useFavorites`)
- `2b875a5` connect/disconnect notifications (setting-gated)
- `2688783` event-driven BT updates (`bluetoothctl` monitor → `bluetooth-changed`; 5s poll → 20s safety net)
- `3c98655` per-app audio volume (pactl sink-inputs)
- `177db7f` configurable nav panels (order / visibility / startup)

**`more-features` batch 2 — new modules + configs:**
- `9d045ba` **System** panel — read-only CPU/RAM/uptime/load (`system.rs`)
- `c8d46f2` **Reload driver** in Devices — modprobe -r/modprobe via pkexec
- `be52ac3` **Power** panel — ppd profiles + battery + brightness (`power.rs`)
- `3b2646d` **Services** panel — systemd list + start/stop/restart (`services.rs`)
- `31a5044` settings: reduce-motion + confirm-destructive

**Verified:** `npm run build` ✅ · `cargo build --release --features custom-protocol` (1m14s) ✅ · `cargo test` **22/22** ✅ · combined app launches clean (window `dmgr — Device Manager`, no errors). Live hardware acceptance (Power/Services on this box, real BT pairing) is the user's to confirm.

**Follow-up (not done):** #9 BT audio A2DP/HSP profile switching (the deferred half of per-app audio).

**Nav-panel system:** panels are registered in `PANEL_META` (settings.ts) **and** `panelAvail` (App.tsx); `effectivePanelOrder()` reconciles a saved order with added/removed panels — add new panels in **both** places + the `View` union + a `{view === "x" && …}` render line.

*(Prior releases this session: v2.1.2 "bluetooth perfection"; v2.1.3 "bluetooth pairing".)*

---

## Current state at a glance

### Build status (v2.1.3, shipped)
- ✅ `npm run build` (frontend) · `cargo build --release --features custom-protocol` (1m13s) · `cargo test` 15/15.
- ✅ Clean-room AUR build from the published `2.1.3-1` tarball — exit 0, well-formed package.
- ✅ Dev release binary launches clean (window `dmgr — Device Manager`, no localhost regression); the new "Discovered devices" + `bt_pair` code is confirmed in the built bundle. (Installed package on this box is still `2.1.2-1` until the user upgrades.)

### Git state — local repo (`/home/isra/projects/dmgr`)
- **Branch:** `main` at **`v2.2.0`** (merge `816c856`, ✅ pushed). `more-features` merged via `--no-ff` (also pushed); `bt-pairing`/`fix-aur-localhost-and-windows` are historical. Tags `v2.1.1`…`v2.2.0` all pushed.
- **`main` tip:** merge commit `64ea732` (✅ pushed) + a `docs(checkpoint)` commit on top recording this shipped state.
- **Tags:** `v2.1.1` `b1df46a`, `v2.1.2` `1c44ff9`, **`v2.1.3`** `64ea732` — all ✅ pushed.
- **Working tree:** clean.
- v2.1.3 landed via: `4ea35ab` (feat: pairing) → `f876714` (version bump) → merge `64ea732`.

### Git state — AUR repo (`/home/isra/aur-dmgr-desktop`)
- **Last commit:** `ca540f4 v2.1.3-1: bump for upstream bluetooth pairing release` (✅ pushed). Working tree clean.
- **Current AUR package:** `2.1.3-1`.
- **Source URL:** `https://github.com/Khinmmad/dmgr/archive/refs/tags/v$pkgver.tar.gz` (auto-bumps with `pkgver`).

### Files changed in the v2.1.3 cycle
- **Feature (`4ea35ab`):** `desktop/src-tauri/src/bluetooth.rs`, `commands.rs`, `lib.rs`, `desktop/src/api.ts`, `desktop/src/components/BluetoothPanel.tsx`.
- **Version bump (`f876714`):** `desktop/package.json`, `desktop/package-lock.json`, `desktop/src-tauri/{Cargo.toml,Cargo.lock,tauri.conf.json}`, `packaging/dmgr-desktop/{PKGBUILD,.SRCINFO}`; AUR `PKGBUILD` + `.SRCINFO` at `2.1.3-1`.
- **Docs:** `CHECKPOINT.md` (this file).

---

## Key technical decisions (don't undo these without thinking)

### Why `--features custom-protocol` is mandatory for production
The Tauri 2 binary, when built without this feature, compiles with `cfg(dev)` and loads `tauri.conf.json`'s `devUrl` (`http://localhost:1420`) at runtime instead of the embedded `frontendDist`. The user gets a blank "localhost failed" window. The Tauri CLI passes this flag automatically; plain `cargo build` does not. **Always** `cargo build --release --features custom-protocol --manifest-path src-tauri/Cargo.toml` for AUR/packaged builds. The `desktop/README.md:38-42` docblock spells this out.

The v2.1.0 GitHub tag was cut *before* this feature was added to `desktop/src-tauri/Cargo.toml`, which is why AUR `2.1.0-4` carried a `prepare()` sed-patch. v2.1.1+ tarballs include the feature natively; the patch is gone.

### Why inline SVG for `⚙` (and now `★`/`☆`)
`⚙` (U+2699 GEAR) and `★`/`☆` (U+2605/2606) are in the "Miscellaneous Symbols" block, which most Linux text fonts skip → render as invisible Tofu boxes. The user is on Hyprland. Emoji (`🔊`, `🔵`, `🧩`, `🎧`) come from Noto Color Emoji and work. **Pattern:** for any single-glyph UI symbol, prefer inline SVG with `currentColor` and a `size` prop. Already converted: settings button, brand, trust button.

### Bluetooth error model
`BtError` (thiserror) serialises to its `Display` string for the TS frontend. The frontend's `invoke<…>` rejects with that string. **Stable contract** — the frontend just shows it in a toast, doesn't pattern-match on it. If the frontend ever needs typed errors, we'll switch to a structured payload (`{kind: "Timeout" | "DaemonDown" | …, message: string}`) — a future enhancement.

### Bluetooth async + timeout pattern
Every `bluetoothctl` (Linux) and `powershell` (Windows) call is `async` via `tokio::process::Command` and wrapped in `tokio::time::timeout`. Timeouts: `QUERY_TIMEOUT=3s`, `ACTION_TIMEOUT=8s`. Tauri 2 commands are `async fn` and inherit the runtime. **Don't** go back to `std::process::Command` — it would block the Tauri runtime and freeze the UI on a hung `bluetoothctl`.

### Tauri 2 + WebKitGTK + Nvidia + Wayland
`src-tauri/src/platform.rs` auto-sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` to avoid a blank window. Override by exporting your own value before launching. Don't remove this workaround.

---

## Bluetooth module — what's left

| Priority | Item | Notes |
|---|---|---|
| P2 | **Event-driven updates** | Replace 5 s polling with `bluetoothctl monitor` → Tauri event `bluetooth-changed`. Needs a background task in the backend (spawn from `lib.rs::run`) and `listen("bluetooth-changed", …)` in `BluetoothPanel.tsx`. |
| P2 | **Device details modal** | Surface battery / type / signal from `bluetoothctl info`. Battery field needs a separate check. |
| ✅ done | **Pair command** | Shipped in v2.1.3 (`4ea35ab`): `pair(mac)` + `bt_pair` + a "Discovered devices" UI section with a Pair button. |
| P2 | **macOS** | `IOBluetooth` via `macos-bluetooth` crate or FFI. Out of scope unless requested. |
| P3 | **Loading skeleton** on first render | |
| P3 | **batched info** via `bluetoothctl --json` | Requires BlueZ 5.66+ (2023). |

### BT module file map
- `desktop/src-tauri/src/bluetooth.rs` — backend (~480 lines, dual `unix_impl` / `win_impl`)
- `desktop/src/components/BluetoothPanel.tsx` — frontend (~220 lines)
- `desktop/src-tauri/src/commands.rs` — Tauri command bindings (line ~112-150 is the BT block)
- `desktop/src/api.ts` — TS API (`bt*` methods)
- `desktop/src/types.ts` — `BtState`, `BtDevice`

---

## Other things to know

### Conventions (from `AGENTS.md`)
- One commit per fix, descriptive messages (`fix(bluetooth): …`, `feat(bluetooth): …`, `refactor(bluetooth): …`).
- Update `AGENTS.md` + `CHECKPOINT.md` if project structure/patterns change.
- The old `PROGRESS.md` reference is stale; `AGENTS.md` is the single source of truth.

### Project structure (dmgr is multi-language; desktop lives in a nested workspace)
```
dmgr/
├── Cargo.toml                  # workspace root (dmgr-core, dmgr-daemon, dmgr-polkit-helper)
├── crates/                     # Rust core engine
├── cli/                        # Python CLI
├── qml/                        # QtQuick UIs (legacy)
├── resources/                  # polkit policy, .desktop file, service units
├── packaging/dmgr-desktop/     # in-repo reference AUR PKGBUILD (source of truth)
├── aur-dmgr-desktop/           # the AUR clone we actually maintain
└── desktop/                    # Tauri + React frontend (this is what we touch)
    ├── src/                    # React/TS
    ├── src-tauri/              # Rust backend (nested workspace!)
    │   ├── Cargo.toml          # [workspace] declared, so the root workspace ignores it
    │   ├── src/
    │   └── tauri.conf.json
    └── package.json
```

### Useful files to read on a fresh session
- `AGENTS.md` — project conventions, build/test commands, known caveats.
- `desktop/README.md` — Tauri build notes (the `--features custom-protocol` gotcha).
- `aur-dmgr-desktop/PKGBUILD` — the AUR pkg we maintain.
- `packaging/dmgr-desktop/PKGBUILD` — in-repo reference (source of truth).
- `desktop/src-tauri/src/bluetooth.rs` — start here for BT work.

### Known caveats
1. **Bus serialization** in dmgr-core: custom `Serialize` impl on the `Bus` enum (not derived). Don't change without a test pass.
2. **Tauri 2 + WebKitGTK DMABUF** workaround in `platform.rs` (above).
3. **Nested workspace** in `desktop/src-tauri/Cargo.toml` declares `[workspace]`, so the repo-root `cargo test` does NOT exercise it. Test the Tauri side via `cargo test --manifest-path desktop/src-tauri/Cargo.toml`.
