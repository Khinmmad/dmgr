# CHECKPOINT — dmgr-desktop

> **You are a fresh agent / new session.** Read this top-to-bottom before touching anything. The TL;DR at the top is the only section you need to start a session; everything below is reference.

Maintained by: **isra (Khinmmad)**. Last updated: **2026-06-14, end of session**.

---

## 🔖 Session handoff — where we stopped

**Done this session, shipped to AUR:**
- `dmgr-desktop 2.1.1-1` — fixes the missing "settings" button (inline SVG `GearIcon` for systems whose font lacks U+2699 `⚙`).

**Done this session, in local branch `fix-aur-localhost-and-windows` (not yet tagged/pushed):**
- Bluetooth module perfection (commits `432bfd5`, `fdaeeb2`):
  - typed `BtError` enum, async/tokio subprocess, timeouts, daemon-down detection, parallel `info()` calls, 6 new Linux unit tests
  - `bt_scan` (10 s discovery) + `bt_remove` (unpair) backend + frontend buttons
  - per-action in-flight guards, daemon-down banner, text-based trust labels (was `★`/`☆`)

**Next concrete step (owned by USER, then AGENT):**
1. User tags the local branch as `v2.1.2` and pushes (commands below).
2. Agent (or user) updates AUR PKGBUILD to `pkgver=2.1.2 pkgrel=1` and pushes.
3. User runs `yay -Syu dmgr-desktop` to install.

```bash
# USER step
cd /home/isra/projects/dmgr
git checkout main && git merge --no-ff fix-aur-localhost-and-windows   # optional
git tag -a v2.1.2 -m "dmgr-desktop 2.1.2 (bluetooth perfection)"
git push origin main fix-aur-localhost-and-windows v2.1.2
```

After `v2.1.2` is on GitHub, the AUR update is a 30-second job:
```bash
# Edit /home/isra/aur-dmgr-desktop/PKGBUILD: pkgver=2.1.1 → 2.1.1, pkgrel=1 (unchanged shape).
# Wait — pkgver becomes 2.1.2. Then:
cd /home/isra/aur-dmgr-desktop
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "v2.1.2-1: bump for upstream bluetooth perfection release"
git push origin master
```

---

## Current state at a glance

### Build status
- ✅ `npm run build` (frontend) passes.
- ✅ `cargo build --release --features custom-protocol` passes.
- ✅ 15 lib tests pass (6 new BT Linux tests + 9 pre-existing).
- ✅ AUR `2.1.1-1` builds and installs cleanly (verified by local `makepkg`).

### Git state — local repo (`/home/isra/projects/dmgr`)
- **Branch:** `fix-aur-localhost-and-windows`
- **HEAD:** `8fd6074 docs(checkpoint): mark BT phases 1-2-3 done, list remaining work`
- **Tagged:** `v2.1.1` at `b1df46a` (✅ pushed to GitHub by user earlier this session)
- **Working tree:** clean
- **Commits past v2.1.1 (5 total):**
  | sha | subject |
  |---|---|
  | `095345b` | docs: add CHECKPOINT.md + link from AGENTS.md |
  | `432bfd5` | refactor(bluetooth): async + timeouts + typed errors + Linux tests |
  | `fdaeeb2` | feat(bluetooth): unpair, scan, daemon-down banner, per-action guards |
  | `8fd6074` | docs(checkpoint): mark BT phases 1-2-3 done, list remaining work |
  *(+ `b1df46a` itself, the GearIcon fix, is what v2.1.1 was tagged on)*

### Git state — AUR repo (`/home/isra/aur-dmgr-desktop`)
- **Last commit:** `1012ddc v2.1.1-1: drop custom-protocol sed patch (now in upstream tarball)` (✅ pushed)
- **Current AUR package:** `2.1.1-1`
- **Source URL:** `https://github.com/Khinmmad/dmgr/archive/refs/tags/v$pkgver.tar.gz` (auto-bumps with `pkgver`)

### Files changed in this session (cumulative)
- **New:** `desktop/src/components/GearIcon.tsx`, `CHECKPOINT.md`
- **Frontend:** `desktop/src/App.tsx`, `desktop/src/api.ts`, `desktop/src/components/BluetoothPanel.tsx`
- **Backend:** `desktop/src-tauri/Cargo.toml` (+ thiserror, + tokio), `desktop/src-tauri/src/bluetooth.rs` (full refactor), `desktop/src-tauri/src/commands.rs`, `desktop/src-tauri/src/lib.rs`
- **Version bumps:** `desktop/package.json`, `desktop/src-tauri/Cargo.toml`, `desktop/src-tauri/tauri.conf.json` (all `2.1.0 → 2.1.1`)
- **AUR:** `aur-dmgr-desktop/PKGBUILD`, `aur-dmgr-desktop/.SRCINFO`
- **Docs:** `AGENTS.md` (one-line pointer to `CHECKPOINT.md`)

---

## Key technical decisions (don't undo these without thinking)

### Why `--features custom-protocol` is mandatory for production
The Tauri 2 binary, when built without this feature, compiles with `cfg(dev)` and loads `tauri.conf.json`'s `devUrl` (`http://localhost:1420`) at runtime instead of the embedded `frontendDist`. The user gets a blank "localhost failed" window. The Tauri CLI passes this flag automatically; plain `cargo build` does not. **Always** `cargo build --release --features custom-protocol --manifest-path src-tauri/Cargo.toml` for AUR/packaged builds. The `desktop/README.md:38-42` docblock spells this out.

The v2.1.0 GitHub tag was cut *before* this feature was added to `desktop/src-tauri/Cargo.toml`, which is why AUR `2.1.0-4` carried a `prepare()` sed-patch. v2.1.1+ tarballs include the feature natively; the patch is gone.

### Why inline SVG for `⚙` (and now `★`/`☆`)
`⚙` (U+2699 GEAR) and `★`/`☆` (U+2605/2606) are in the "Miscellaneous Symbols" block, which most Linux text fonts skip → render as invisible Tofu boxes. The user is on Hyprland. Emoji (`🔊`, `🔵`, `🧩`, `🎧`) come from Noto Color Emoji and work. **Pattern:** for any single-glyph UI symbol, prefer inline SVG with `currentColor` and a `size` prop. Already converted: settings button, brand, trust button.

### Bluetooth error model
`BtError` (thiserror) serialises to its `Display` string for the TS frontend. The frontend's `invoke<…>` rejects with that string. **Stable contract** — the frontend just shows it in a toast, doesn't pattern-match on it. If the frontend ever needs typed errors, we'll switch to a structured payload (`{kind: "Timeout" | "DaemonDown" | …, message: string}`) — that's a v2.1.3 thing.

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
| P2 | **Pair command** | `bluetoothctl pair <mac>`. Needs a "discovered devices" section in the UI. |
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
