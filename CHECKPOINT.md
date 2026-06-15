# CHECKPOINT — dmgr-desktop

> **You are a fresh agent / new session.** Read this top-to-bottom before touching anything. The TL;DR at the top is the only section you need to start a session; everything below is reference.

Maintained by: **isra (Khinmmad)**. Last updated: **2026-06-15, v2.2.0 shipped & verified**.

---

## 🔖 Session handoff — where we stopped

**v2.2.0 SHIPPED, INSTALLED & verified this session.** 🎉 The `more-features` batch (13
features below) merged to `main`, tagged **`v2.2.0`** (merge `816c856`), pushed to GitHub;
AUR at **`2.2.0-1`** (`261658a`); **installed `2.2.0-1` on this box** (`pacman -Qkk` 21/0,
GUI launches clean). Nothing pending for the release.

**▶ NEXT SESSION — start here:** the one open item is **#9 — BT audio A2DP/HSP profile
switching** (the deferred half of "advanced audio"): parse `pactl list cards` for card
profiles + the active one, add a `set-card-profile` command, and a profile selector in
`AudioPanel.tsx` for Bluetooth headsets (A2DP high-quality vs HSP/HFP hands-free). Ship as
`2.2.1`. Working tree clean, on `main` — branch off `main`. To install a built pkg
hands-free: start a polkit agent, then `pkexec pacman -U …` (see the `dmgr-desktop-verify-env`
memory). Otherwise the project is in a clean, fully-released state.

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

### Build status (v2.2.0, shipped & installed)
- ✅ `npm run build` (frontend) · `cargo build --release --features custom-protocol` (1m14s) · `cargo test` **22/22**.
- ✅ Clean-room AUR build from the published `2.2.0-1` tarball — exit 0, well-formed package (binary + polkit-helper + policy).
- ✅ Installed `2.2.0-1` on this box (`pacman -Qkk` 21/0); the installed app launches clean (window `dmgr — Device Manager`, no localhost regression).

### Git state — local repo (`/home/isra/projects/dmgr`)
- **Branch:** `main` at **`v2.2.0`** (merge `816c856` + a `docs(checkpoint)` commit on top, all ✅ pushed; in sync with `origin/main`). `more-features` merged via `--no-ff` (also pushed); `bt-pairing` / `fix-aur-localhost-and-windows` are historical.
- **Tags:** `v2.1.1` … **`v2.2.0`** — all ✅ pushed (`v2.2.0` at merge `816c856`).
- **Working tree:** clean.
- v2.2.0 = the 13-commit `more-features` batch + `9bd6d76` (2.1.3→2.2.0 bump) → merge `816c856`.

### Git state — AUR repo (`/home/isra/aur-dmgr-desktop`)
- **Last commit:** `261658a v2.2.0-1: bump for upstream panels + customization release` (✅ pushed). Working tree clean.
- **Current AUR package:** `2.2.0-1`.
- **Source URL:** `https://github.com/Khinmmad/dmgr/archive/refs/tags/v$pkgver.tar.gz` (auto-bumps with `pkgver`).

### Release recipe (proven 3× this session: 2.1.2 → 2.1.3 → 2.2.0)
1. **Bump version** (grep the old version first to catch everything; do NOT touch the 3rd-party `ms`/`derive_more`/`rustc-hash` matches): `desktop/package.json`, `desktop/package-lock.json` (only the 2 top self-version lines), `desktop/src-tauri/{Cargo.toml, tauri.conf.json, Cargo.lock` (the `dmgr-desktop` entry only)`}`, `packaging/dmgr-desktop/PKGBUILD` (+ `makepkg --printsrcinfo > .SRCINFO`). Commit on a feature branch.
2. **Verify:** `cargo build --release --features custom-protocol` · `cargo test` · `npm run build` (run from `desktop/`).
3. **Ship:** `git switch main && git merge --no-ff <branch>` → `git tag -a vX.Y.Z -m …` → `git push origin main <branch> vX.Y.Z` (`GIT_SSH_COMMAND="ssh -o BatchMode=yes"`). Verify tag on remote + `curl -sI` the tarball (expect 200).
4. **AUR** (`/home/isra/aur-dmgr-desktop`, only AFTER the GitHub tag exists): edit `pkgver`, `makepkg --printsrcinfo > .SRCINFO`, commit, `git push origin master`.
5. **Clean-room verify + install:** clone AUR to `/tmp`, `makepkg -f --noconfirm` (no sudo). Install hands-free: start a polkit agent then `pkexec pacman -U <pkg>` — see the `dmgr-desktop-verify-env` memory.

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
| ✅ done | **Pair command** | v2.1.3 (`4ea35ab`): `pair`/`bt_pair` + "Discovered devices" section. |
| ✅ done | **Device details modal** | v2.2.0 (`05feaa4`): battery/RSSI/type modal + auto-connect; battery inline. |
| ✅ done | **Event-driven updates** | v2.2.0 (`2688783`): `bluetoothctl` monitor → `bluetooth-changed`; 5 s poll → 20 s safety net. |
| **▶ P1 next** | **A2DP/HSP profile switching** (#9) | `pactl list cards` → profiles + active; add `set-card-profile`; profile selector in `AudioPanel.tsx` for BT headsets. Ship as 2.2.1. |
| P2 | **macOS** | `IOBluetooth` via `macos-bluetooth` crate or FFI. Out of scope unless requested. |
| P3 | **Loading skeleton** on first render | |
| P3 | **batched info** via `bluetoothctl --json` | Requires BlueZ 5.66+ (2023). |

### BT module file map
- `desktop/src-tauri/src/bluetooth.rs` — backend (~830 lines, dual `unix_impl` / `win_impl`; `monitor` spawns the event task)
- `desktop/src/components/BluetoothPanel.tsx` — frontend (~370 lines; details modal, search, favorites, aliases)
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
