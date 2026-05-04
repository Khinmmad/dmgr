# AGENTS.md — dmgr project guide for AI agents

## Project overview

`dmgr` is a multi-language device manager for Arch Linux, similar to Windows Device Manager. It detects and manages hardware devices (USB, PCIe, audio, input, block, GPU, network) via sysfs and udev.

**Repo**: https://github.com/Khinmmad/dmgr  
**Branch**: `main`

## Architecture

```
Rust (60%)                 QML/JS (30%)            Python (8%)
┌──────────┐    DBus     ┌──────────────┐         ┌──────────┐
│dmgr-core │◄──────────►│ dmgr-daemon  │◄───────►│ dmgr CLI │
│ sysfs    │            │  (zbus 4.x)  │         │(dasbus)  │
│ udev     │            └──────┬───────┘         └──────────┘
│ control  │                  │
└──────────┘            ┌─────┴──────┐
                        │  QML UIs   │
                        │ panel +    │
                        │ standalone │
                        └────────────┘
```

- **Core engine** (`crates/dmgr-core`): Rust library for sysfs parsing, udev monitoring, driver control
- **DBus daemon** (`crates/dmgr-daemon`): zbus-based session bus service (`org.dmgr.DeviceManager`)
- **Privileged helper** (`crates/dmgr-polkit-helper`): minimal binary invoked via `pkexec` for root operations
- **QML UI** (`qml/`): QtQuick 6 interfaces (QuickShell panel + standalone window)
- **CLI** (`cli/`): Python CLI using dasbus + rich

## Directory layout

```
dmgr/
├── Cargo.toml                  # Workspace root (dmgr-core, dmgr-daemon, dmgr-polkit-helper)
├── crates/
│   ├── dmgr-core/              # Rust lib: device.rs, sysfs.rs, udev.rs, control.rs, properties.rs
│   │   └── tests/device_tests.rs  # 10 integration tests
│   ├── dmgr-daemon/            # Rust bin: DBus service (main.rs)
│   └── dmgr-polkit-helper/     # Rust bin: root helper (main.rs)
├── qml/                        # QtQuick QML (shared between QuickShell & standalone)
│   ├── components/             # Reusable: DeviceTree, DeviceDetail, DeviceControls, etc.
│   ├── dbus/                   # DeviceManagerProxy.qml (singleton DBus client)
│   ├── theme/                  # DmgrTheme.qml (dark theme colors)
│   ├── icons/                  # 8 SVG icons
│   ├── dmgr-panel.qml          # QuickShell.Panel entry
│   └── dmgr-standalone.qml     # ApplicationWindow entry
├── cli/dmgr/                   # Python CLI
│   ├── __main__.py             # argparse entry (list, info, search, bind, unbind, property, watch, refresh, drivers)
│   ├── client.py               # DMgrClient with dasbus
│   └── formatters.py           # Rich tables / JSON output
├── resources/
│   ├── dmgr.desktop, dmgr-daemon.service, dmgr-daemon.desktop
│   ├── org.dmgr.DeviceManager.policy  # Polkit actions
│   └── quickshell/
│       ├── metadata.json        # QuickShell module metadata
│       └── DeviceManager.qml    # Sidebar component for QuickShell
├── scripts/                    # install.sh, uninstall.sh
├── packaging/                  # PKGBUILD for AUR
├── PROJECT.md                  # Full specification
├── PROGRESS.md                 # Development log
└── README.md                   # User-facing docs
```

## Build & test commands

```bash
# Check compilation
cargo check

# Full test suite (12 tests)
cargo test

# Release build (outputs to target/release/)
cargo build --release

# Lint QML files
qmllint qml/**/*.qml

# Install system-wide (requires sudo)
sudo bash scripts/install.sh

# Install Python CLI (requires dasbus, rich)
pip install --user ./cli
```

## Key dependencies

| Crate/package | Version | Purpose |
|---|---|---|
| `zbus` | 4.x | DBus server (pure Rust) |
| `udev` | 0.9 | Device events monitoring |
| `tokio` | 1.x | Async runtime for daemon |
| `serde` / `serde_json` | 1.x | Serialization |
| `thiserror` | 1.x | Error derive macro |
| `dasbus` (Python) | 1.7+ | DBus client for CLI |
| `rich` (Python) | 13+ | Terminal output |
| `qt6-declarative` | 6.x | QML runtime (system) |

## Known caveats

1. **Bus serialization**: `Bus` enum has a custom `Serialize` impl (not derived) because the default serde enum serialization produces `{"Unknown":"string"}` for `Bus::Unknown`. The custom impl always outputs a string.

2. **Device status**: `DeviceStatus::from_str()` defaults to `Online` for unrecognized values. This was a deliberate fix because 230/358 devices had non-standard `power/runtime_status` values.

3. **Udev thread**: The udev monitor runs in a separate `std::thread` (not a tokio task) because `mpsc::recv()` blocks. The thread spawns its own `tokio::runtime` for emitting DBus signals.

4. **DBus NameTaken**: If the daemon is already running, it exits with code 1 and clear instructions. Launch via `systemctl --user start dmgr-daemon` instead.

5. **QML Process**: The QuickShell sidebar component uses `Quickshell.Io.Process` with `/usr/bin/busctl` (full path) to query the daemon. Uses `ListModel` instead of JS arrays for reliable ListView binding.

6. **JSON escaping**: `busctl call` outputs strings with `\"` escaped quotes. The QML component handles this with `.replace(/\\"/g, '"')`.

## DBus API reference

**Bus**: Session | **Name**: `org.dmgr.DeviceManager` | **Path**: `/org/dmgr/DeviceManager`

| Method | Signature | Returns |
|---|---|---|
| `GetAllDevices` | — | `s` (JSON array) |
| `GetDevice` | `s` dev_id | `s` (JSON) |
| `GetDevicesByBus` | `s` bus | `s` (JSON array) |
| `GetDevicesByFilter` | `s` query | `s` (JSON array) |
| `GetAvailableDrivers` | `s` dev_id | `as` |
| `BindDriver` | `s` dev_id, `s` driver | `b` |
| `UnbindDriver` | `s` dev_id | `b` |
| `SetProperty` | `s` dev_id, `s` attr, `s` val | `b` |
| `Refresh` | — | `u` count |

| Signal | Args |
|---|---|
| `DeviceAdded` | `u` count |
| `DeviceRemoved` | `u` count |
| `DeviceChanged` | `s` dev_id |
| `ScanFinished` | `u` count |

## Git workflow

- 5 commits on `main`
- Always commit + push each fix separately with descriptive messages
- Update `PROGRESS.md` after significant changes
- Update this `AGENTS.md` if project structure/patterns change
