# CHECKPOINT — dmgr-desktop

> **You are a fresh agent / new session.** Read this top-to-bottom before touching anything. The TL;DR at the top is the only section you need to start a session; everything below is reference.

Maintained by: **isra (Khinmmad)**. Last updated: **2026-06-14, v2.1.2 release prep**.

---

## 🔖 Session handoff — where we stopped

**Done & verified this session (local branch `fix-aur-localhost-and-windows`):**
- Bluetooth module perfection (commits `432bfd5`, `fdaeeb2`): typed `BtError`, async/tokio + timeouts, daemon-down detection, parallel `info()`, `bt_scan` (10 s) + `bt_remove` (unpair), per-action in-flight guards, daemon-down banner, text trust labels (was `★`/`☆`), 6 new Linux tests.
- **Version bump `2.1.1 → 2.1.2`** (commit `61ce4ac`): `package.json`, `tauri.conf.json`, `src-tauri/Cargo.toml`, and the **dmgr-desktop** `Cargo.lock` entry only (`derive_more` 2.1.1 / `rustc-hash` 2.1.2 are unrelated 3rd-party crates — left untouched). Also resynced `packaging/dmgr-desktop/PKGBUILD` to `2.1.2-1` (was stale at `2.1.0-3`).
- **Verified at 2.1.2:** `npm run build` ✅ · `cargo build --release --features custom-protocol` ✅ (1m23s) · `cargo test` ✅ 15/15.
- **AUR repo PRE-STAGED but intentionally NOT committed/pushed** (`/home/isra/aur-dmgr-desktop`): `PKGBUILD` → `pkgver=2.1.2 pkgrel=1`, `.SRCINFO` regenerated. Held back on purpose — the AUR push must come **after** the GitHub tag, or the `v2.1.2.tar.gz` source won't resolve.

**Next concrete step:**
1. **USER** — tag `v2.1.2` and push (the version bump is already committed at the branch tip):
   ```bash
   cd /home/isra/projects/dmgr
   git checkout main && git merge --no-ff fix-aur-localhost-and-windows   # optional
   git tag -a v2.1.2 -m "dmgr-desktop 2.1.2 (bluetooth perfection)"
   git push origin main fix-aur-localhost-and-windows v2.1.2
   ```
2. **USER or AGENT** — once `v2.1.2` is on GitHub, finish the AUR push (PKGBUILD + .SRCINFO already edited):
   ```bash
   cd /home/isra/aur-dmgr-desktop
   git add PKGBUILD .SRCINFO
   git commit -m "v2.1.2-1: bump for upstream bluetooth perfection release"
   git push origin master
   ```
3. **USER** — `yay -Syu dmgr-desktop` to install.

---

## Current state at a glance

### Build status (verified at 2.1.2)
- ✅ `npm run build` (frontend) passes.
- ✅ `cargo build --release --features custom-protocol --manifest-path desktop/src-tauri/Cargo.toml` passes (1m23s).
- ✅ 15 lib tests pass (6 BT Linux tests + 9 pre-existing).
- ✅ AUR `2.1.1-1` built & installed cleanly previously; `2.1.2-1` PKGBUILD prepared (not yet pushed — pending GitHub tag).

### Git state — local repo (`/home/isra/projects/dmgr`)
- **Branch:** `fix-aur-localhost-and-windows`
- **HEAD:** the `docs(checkpoint)` commit recording this v2.1.2 prep (tip of branch).
- **Tagged:** `v2.1.1` at `b1df46a` (✅ pushed). `v2.1.2` **not yet created** — that's the USER's next step.
- **Working tree:** clean.
- **Commits past v2.1.1:**
  | sha | subject |
  |---|---|
  | `095345b` | docs: add CHECKPOINT.md + link from AGENTS.md (✅ pushed) |
  | `432bfd5` | refactor(bluetooth): async + timeouts + typed errors + Linux tests |
  | `fdaeeb2` | feat(bluetooth): unpair, scan, daemon-down banner, per-action guards |
  | `8fd6074` | docs(checkpoint): mark BT phases 1-2-3 done |
  | `86d886e` | docs(checkpoint): clean final handoff |
  | `61ce4ac` | chore(release): bump dmgr-desktop to 2.1.2 |
  | *(tip)* | docs(checkpoint): prep v2.1.2 release handoff |
  *(everything from `432bfd5` onward is unpushed; `b1df46a`, the GearIcon fix, is where v2.1.1 was tagged.)*

### Git state — AUR repo (`/home/isra/aur-dmgr-desktop`)
- **Last commit:** `1012ddc v2.1.1-1: drop custom-protocol sed patch (now in upstream tarball)` (✅ pushed).
- **Working tree:** **dirty** — `PKGBUILD` (`pkgver=2.1.2 pkgrel=1`) + `.SRCINFO` edited, **not committed**. Commit/push only after `v2.1.2` is on GitHub.
- **Source URL:** `https://github.com/Khinmmad/dmgr/archive/refs/tags/v$pkgver.tar.gz` (auto-bumps with `pkgver`).

### Files changed in the v2.1.2 cycle
- **Version bumps (`2.1.1 → 2.1.2`):** `desktop/package.json`, `desktop/src-tauri/Cargo.toml`, `desktop/src-tauri/tauri.conf.json`, `desktop/src-tauri/Cargo.lock` (dmgr-desktop entry).
- **Packaging:** `packaging/dmgr-desktop/PKGBUILD` resynced to `2.1.2-1`; `aur-dmgr-desktop/PKGBUILD` + `.SRCINFO` staged to `2.1.2-1` (uncommitted).
- **Docs:** `CHECKPOINT.md` (this file).
- *(BT source files — `bluetooth.rs`, `commands.rs`, `lib.rs`, `BluetoothPanel.tsx`, `api.ts`, `Cargo.toml` deps — were changed earlier in commits `432bfd5`/`fdaeeb2`.)*

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
