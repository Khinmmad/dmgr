# dmgr — Gestor de Dispositivos para Arch Linux

Herramienta dual para administrar dispositivos conectados en Arch Linux:
1. **Panel QML** nativo en QuickShell (QtQuick)
2. **App standalone** con el mismo código QML
3. **CLI** en Python para consultas rápidas

Backend potente en Rust con DBus session bus como puente universal.

---

## Arquitectura Multi-Lenguaje

```
┌──────────────────────────────────────────────────────────────┐
│                      RUST (60%)                              │
│  ┌────────────┐  ┌───────────┐  ┌─────────────────────────┐ │
│  │ dmgr-core  │  │  dmgr-    │  │  dmgr-polkit-helper     │ │
│  │ (sysfs,    │  │  daemon   │  │  (bind/unbind root)     │ │
│  │  udev,     │  │  (DBus)   │  │                         │ │
│  │  control)  │  │           │  │  Lenguajes: Rust        │ │
│  └─────┬──────┘  └─────┬─────┘  └─────────────────────────┘ │
│        │               │                                     │
│        │        ┌──────┴───────┐                             │
│        │        │   DBus       │  org.dmgr.DeviceManager     │
│        │        │   Session    │                             │
│        │        └──────┬───────┘                             │
├────────┼───────────────┼─────────────────────────────────────┤
│   QML + JavaScript (30%)  ── FRONTENDS ──                   │
│        │               │                                     │
│  ┌─────┴────────┐  ┌───┴─────────────┐                      │
│  │ dmgr-panel   │  │  dmgr-standalone│   Artefactos QML:    │
│  │ (QuickShell) │  │  (qmlscene app) │   - Panel nativo     │
│  └──────────────┘  └─────────────────┘   - App independiente│
│        │               │              Comparten código QML  │
│        └───────┬───────┘              con DBus Qt bindings  │
│        ┌───────┴───────┐                                     │
│        │  DBus/QtQuick │  Qt.labs.dbus / DBusConnection     │
│        └───────────────┘                                     │
├──────────────────────────────────────────────────────────────┤
│                      PYTHON (8%)                             │
│  ┌────────────────┐  ┌──────────────────────────┐          │
│  │   dmgr-cli     │  │  scripts/install.py       │          │
│  │  (dasbus/jeep) │  │  (instalación/update)     │          │
│  └────────────────┘  └──────────────────────────┘          │
├──────────────────────────────────────────────────────────────┤
│                      SHELL (2%)                              │
│  ┌──────────────────────────────────────────────┐          │
│  │  PKGBUILD, dmgr-daemon.service, dmgr.desktop │          │
│  └──────────────────────────────────────────────┘          │
└──────────────────────────────────────────────────────────────┘
```

### Por qué cada lenguaje

| Lenguaje | Rol | Justificación |
|---|---|---|
| **Rust** | Core engine + daemon + helper root | Seguridad de memoria, zero-cost abstractions, acceso directo a sysfs, `zbus` puro Rust para DBus |
| **QML + JS** | Toda la UI | QuickShell ES QML — usar QML para el frontend da integración nativa, compartir código entre panel y app standalone |
| **Python** | CLI + scripts | Fácil de modificar por usuarios Arch, librerías DBus maduras, ideal para tooling |
| **Shell** | PKGBUILD, systemd units | Estándar de facto en Arch Linux |

---

## Estructura del proyecto

```
dmgr/
├── PROJECT.md
├── Cargo.toml                          # Workspace: dmgr-core + daemon + polkit-helper
├── setup.py / pyproject.toml           # CLI Python + scripts
│
├── crates/
│   ├── dmgr-core/                      # Librería central RUST
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── device.rs               # Struct Device, Bus, DeviceStatus
│   │       ├── sysfs.rs                # Parser /sys/bus/*, /sys/class/*
│   │       ├── udev.rs                 # Listener udev con tokio
│   │       ├── control.rs              # Bind/unbind (lógica pura)
│   │       ├── properties.rs           # Lectura/escritura de atributos
│   │       └── error.rs                # Tipos de error
│   │
│   ├── dmgr-daemon/                    # Servicio DBus RUST
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs                 # zbus Connection + serve
│   │       └── dbus_iface.rs           # Trait org.dmgr.DeviceManager
│   │
│   └── dmgr-polkit-helper/            # Helper root RUST (pkexec)
│       ├── Cargo.toml
│       └── src/
│           └── main.rs                 # bind | unbind | set
│
├── qml/                                # QML + JS (compartido)
│   ├── qmldir
│   ├── dmgr-panel.qml                  # Panel integrado QuickShell
│   ├── dmgr-standalone.qml             # App independiente
│   ├── components/
│   │   ├── DeviceTree.qml
│   │   ├── DeviceDetail.qml
│   │   ├── DeviceControls.qml
│   │   ├── SearchBar.qml
│   │   ├── PropertyEditor.qml
│   │   └── StatusIndicator.qml
│   ├── dbus/
│   │   └── DeviceManagerProxy.qml
│   ├── theme/
│   │   └── DmgrTheme.qml
│   └── icons/
│       ├── usb.svg, pci.svg, audio.svg, input.svg,
│       ├── block.svg, gpu.svg, network.svg
│
├── cli/                                # PYTHON
│   ├── dmgr/
│   │   ├── __init__.py
│   │   ├── __main__.py
│   │   ├── client.py                   # DBus client
│   │   └── formatters.py
│   └── pyproject.toml
│
├── resources/
│   ├── dmgr.desktop
│   ├── dmgr-daemon.service
│   ├── dmgr-daemon.desktop
│   ├── org.dmgr.DeviceManager.policy
│   └── quickshell/
│       └── metadata.json
│
├── scripts/
│   ├── install.sh
│   ├── install.py
│   └── uninstall.sh
│
└── packaging/
    └── PKGBUILD
```

---

## API DBus — `org.dmgr.DeviceManager`

**Bus**: Session

| Método | Args | Retorno | Descripción |
|---|---|---|---|
| `GetAllDevices` | — | `s` (JSON array) | Lista todos los dispositivos |
| `GetDevice` | `s` (dev_id) | `s` (JSON) | Detalles de un dispositivo |
| `GetDevicesByBus` | `s` (bus) | `s` (JSON array) | Filtra por bus |
| `GetDevicesByFilter` | `s` (query) | `s` (JSON array) | Búsqueda textual |
| `GetAvailableDrivers` | `s` (dev_id) | `as` | Drivers compatibles |
| `BindDriver` | `s` (dev_id), `s` (driver) | `b` | Vincula driver (pkexec) |
| `UnbindDriver` | `s` (dev_id) | `b` | Desvincula driver (pkexec) |
| `SetProperty` | `s` (dev_id), `s` (attr), `s` (val) | `b` | Edita propiedad sysfs |
| `Refresh` | — | — | Re-escanear todos los buses |

| Señal | Args | Descripción |
|---|---|---|
| `DeviceAdded` | `s` (JSON device) | Dispositivo conectado |
| `DeviceRemoved` | `s` (dev_id) | Dispositivo desconectado |
| `DeviceChanged` | `s` (dev_id), `s` (JSON props) | Propiedad cambiada |
| `ScanFinished` | `u` (count) | Escaneo completado |

---

## Estructura de datos Device (JSON en DBus)

```json
{
    "id": "pci-0000:00:14.0",
    "name": "xHCI Host Controller",
    "bus": "Pci",
    "bus_id": "0000:00:14.0",
    "vendor": "Intel Corporation",
    "vendor_id": "8086",
    "model": "Alder Lake PCH USB 3.2 xHCI",
    "model_id": "51ed",
    "driver": "xhci_hcd",
    "status": "Online",
    "path": "/sys/devices/pci0000:00/0000:00:14.0",
    "parent": "pci-0000:00:00.0",
    "children": ["usb-1", "usb-2"],
    "interfaces": [
        {"name": "usb1", "class": "UsbHost"},
        {"name": "usb2", "class": "UsbHost"}
    ],
    "properties": {
        "power/control": "auto",
        "power/runtime_status": "active",
        "removable": "fixed",
        "iommu_group": "12"
    },
    "editable_properties": [
        "power/control"
    ]
}
```

---

## Uso de dmgr-cli

```bash
dmgr list                        # Lista todos los dispositivos
dmgr list --bus usb              # Solo USB
dmgr list --bus pci              # Solo PCIe
dmgr list --bus audio            # Solo audio
dmgr list --status unbound       # Solo sin driver
dmgr info <dev-id>               # Detalles completos
dmgr info usb-1-2                # Dispositivo específico
dmgr search "Realtek"            # Búsqueda textual
dmgr bind <dev-id> <driver>      # Vincular driver (requiere auth)
dmgr unbind <dev-id>             # Desvincular driver
dmgr property get <dev-id> power/control
dmgr property set <dev-id> power/control on
dmgr watch                       # Monitorear eventos udev en vivo
dmgr daemon start                # Iniciar daemon
dmgr daemon status               # Ver estado del daemon
```

---

## Dependencias

### Rust (en Cargo.toml)
| Crate | Versión | Propósito |
|---|---|---|
| `zbus` | 4.x | DBus server (puro Rust) |
| `udev` | 0.8 | Eventos udev |
| `tokio` | 1.x | Async runtime |
| `serde` / `serde_json` | 1.x | Serialización |
| `serde_with` | 3.x | Serialización avanzada |
| `thiserror` | 1.x | Derive Error |
| `log` / `env_logger` | 0.4 | Logging |
| `dirs` | 5.x | XDG paths |
| `nix` | 0.29 | Syscalls (opcional) |

### Sistema (pacman)
| Paquete | Propósito |
|---|---|
| `rust` + `cargo` | Compilación |
| `qt6-declarative` | Runtime QML standalone |
| `qt6-base` | Qt base |
| `polkit` | Autenticación |
| `systemd` | Servicio de usuario |

### Python (pip / pacman)
| Paquete | Propósito |
|---|---|
| `dasbus` | DBus client |
| `rich` | Terminal output |
| `click` | CLI framework |

---

## Fases de desarrollo

| Fase | Descripción | Estado |
|---|---|---|
| **Fase 1** | Workspace + dmgr-core (device, sysfs, udev, control, error) | ⬜ |
| **Fase 2** | dmgr-daemon con zbus DBus | ⬜ |
| **Fase 3** | dmgr-polkit-helper + policy | ⬜ |
| **Fase 4** | Componentes QML base (DeviceTree, DeviceDetail, DBus proxy) | ⬜ |
| **Fase 5** | dmgr-panel.qml (QuickShell) + dmgr-standalone.qml | ⬜ |
| **Fase 6** | dmgr-cli en Python con dasbus | ⬜ |
| **Fase 7** | Iconos SVG + tema + animaciones | ⬜ |
| **Fase 8** | Scripts instalación, .desktop, systemd, PKGBUILD | ⬜ |
| **Fase 9** | Tests + docs | ⬜ |

---

## Licencia

MIT
