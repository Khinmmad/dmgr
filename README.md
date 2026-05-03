# dmgr — Gestor de Dispositivos para Arch Linux

Administrador de dispositivos estilo Windows para Arch Linux. Detecta dispositivos USB, PCIe, audio, input, block, GPU, red, etc. Permite ver propiedades, bind/unbind drivers y editar atributos vía sysfs.

## Arquitectura

```
┌──────────┐     DBus      ┌─────────────┐     QML      ┌──────────────────┐
│ dmgr-core │◄────────────►│ dmgr-daemon │◄────────────►│ dmgr-panel (UI)  │
│  (Rust)   │              │   (Rust)    │              │ dmgr-standalone  │
└──────────┘              └─────────────┘              │  (QML/QtQuick)   │
                                │                       └──────────────────┘
                                │ DBus
                    ┌───────────┴───────────┐
                    │      dmgr (CLI)       │
                    │       (Python)        │
                    └───────────────────────┘
```

- **Rust**: Core engine (sysfs scanner, udev monitor, driver control)
- **QML/QtQuick**: Interfaz gráfica (QuickShell panel + standalone app)
- **Python**: CLI con `dasbus` + `rich`
- **DBus**: `org.dmgr.DeviceManager` — session bus, todos los frontends se comunican vía este protocolo

## Requisitos

```bash
# Dependencias de sistema (Arch Linux)
sudo pacman -S rust cargo qt6-declarative qt6-base polkit systemd python python-dasbus python-rich

# Para QuickShell (si usas ese shell)
yay -S quickshell-git
```

## Instalación

```bash
git clone https://github.com/Khinmmad/dmgr.git
cd dmgr

# Build release
cargo build --release

# Instalar (requiere sudo para copiar a /usr)
sudo bash scripts/install.sh

# Instalar CLI Python
pip install --user ./cli
```

## Configuración

### 1. Iniciar el daemon

```bash
# Habilitar e iniciar el daemon como servicio de usuario
systemctl --user enable --now dmgr-daemon

# Verificar que está corriendo
systemctl --user status dmgr-daemon

# Logs
journalctl --user -u dmgr-daemon -f
```

### 2. Verificar DBus

```bash
# Listar métodos disponibles
busctl --user introspect org.dmgr.DeviceManager /org/dmgr/DeviceManager

# Llamar al método GetAllDevices
busctl --user call org.dmgr.DeviceManager /org/dmgr/DeviceManager \
  org.dmgr.DeviceManager GetAllDevices
```

### 3. Lanzar la UI

```bash
# App standalone QtQuick
qml6 /usr/share/dmgr/qml/dmgr-standalone.qml

# O desde el lanzador de aplicaciones (dmgr.desktop)
```

### 4. Usar la CLI

```bash
dmgr list                          # Listar todos los dispositivos
dmgr list --bus usb                # Solo USB
dmgr list --bus pci                # Solo PCIe
dmgr info pci-0000:00:14.0        # Detalles de un dispositivo
dmgr search Intel                  # Buscar dispositivos
dmgr drivers pci-0000:00:14.0     # Ver drivers disponibles
dmgr watch                         # Monitoreo en vivo (polling)
dmgr refresh                       # Re-escanear

# Operaciones privilegiadas (requieren polkit)
dmgr bind pci-0000:00:14.0 xhci_hcd   # Vincular driver
dmgr unbind pci-0000:00:14.0          # Desvincular driver
dmgr property get pci-0000:00:14.0 power/control
dmgr property set pci-0000:00:14.0 power/control on
```

### 5. Integración QuickShell

El módulo se instala automáticamente en `/usr/share/quickshell/modules/dmgr/`. Si no aparece en tu shell:

```bash
# Verificar instalación
ls /usr/share/quickshell/modules/dmgr/

# Si tu QuickShell usa otra ruta, copiar manualmente:
cp -r /usr/share/quickshell/modules/dmgr ~/.config/quickshell/modules/
```

## API DBus

**Bus**: Session  
**Nombre**: `org.dmgr.DeviceManager`  
**Path**: `/org/dmgr/DeviceManager`

| Método | Args | Retorno |
|---|---|---|
| `GetAllDevices` | — | `s` (JSON) |
| `GetDevice` | `s` dev_id | `s` (JSON) |
| `GetDevicesByBus` | `s` bus | `s` (JSON) |
| `GetDevicesByFilter` | `s` query | `s` (JSON) |
| `GetAvailableDrivers` | `s` dev_id | `as` |
| `BindDriver` | `s` dev_id, `s` driver | `b` |
| `UnbindDriver` | `s` dev_id | `b` |
| `SetProperty` | `s` dev_id, `s` attr, `s` val | `b` |
| `Refresh` | — | `u` |

**Señales**: `DeviceAdded`, `DeviceRemoved`, `DeviceChanged`, `ScanFinished`

## Desinstalación

```bash
# Detener daemon
systemctl --user disable --now dmgr-daemon

# Desinstalar binarios y recursos
sudo bash scripts/uninstall.sh

# Desinstalar CLI
pip uninstall dmgr
```

## Estructura del proyecto

```
dmgr/
├── crates/
│   ├── dmgr-core/          # Librería Rust: scanner sysfs, udev, control drivers
│   ├── dmgr-daemon/        # Servicio DBus (zbus)
│   └── dmgr-polkit-helper/ # Helper privilegiado (pkexec)
├── qml/                    # Interfaz QML (QuickShell + standalone)
│   ├── components/         # DeviceTree, DeviceDetail, DeviceControls...
│   ├── dbus/               # Proxy DBus para QML
│   ├── theme/              # Tema oscuro
│   └── icons/              # SVG por tipo de bus
├── cli/                    # CLI Python (dasbus + rich)
├── resources/              # .desktop, .service, polkit policy
├── scripts/                # install.sh / uninstall.sh
├── packaging/              # PKGBUILD para AUR
├── PROJECT.md              # Especificación completa
└── PROGRESS.md             # Log de desarrollo
```

## Licencia

MIT
