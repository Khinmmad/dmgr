"""Output formatters for dmgr CLI."""

import json
import sys

try:
    from rich.console import Console
    from rich.table import Table
    from rich.text import Text
    from rich.tree import Tree
    HAS_RICH = True
except ImportError:
    HAS_RICH = False


STATUS_COLORS = {
    "Online": "green",
    "Offline": "red",
    "Suspended": "yellow",
    "Unbound": "grey",
    "Error": "orange1",
    "Unknown": "grey",
}

BUS_EMOJI = {
    "Pci": "🔌",
    "Usb": "🔗",
    "Audio": "🔊",
    "Input": "⌨️",
    "Block": "💾",
    "Drm": "🖥️",
    "Net": "🌐",
    "Hid": "🖱️",
    "Tty": "📟",
    "Power": "🔋",
}


def print_devices_table(devices: list, title: str = "Devices"):
    if not HAS_RICH:
        _print_plain_table(devices)
        return

    console = Console()
    table = Table(title=title)

    table.add_column("ID", style="dim")
    table.add_column("Name")
    table.add_column("Bus")
    table.add_column("Driver", style="cyan")
    table.add_column("Status")

    for dev in devices:
        bus_name = dev.get("bus", "?")
        status = dev.get("status", "Unknown")
        status_style = STATUS_COLORS.get(status, "grey")
        table.add_row(
            dev.get("id", "")[:30],
            dev.get("name", dev.get("id", "")),
            f"{BUS_EMOJI.get(bus_name, '')} {bus_name}",
            dev.get("driver") or "(none)",
            f"[{status_style}]{status}[/{status_style}]",
        )

    console.print(table)


def _print_plain_table(devices: list):
    header = f"{'ID':<35} {'Name':<30} {'Bus':<10} {'Driver':<20} {'Status':<12}"
    print(header)
    print("-" * len(header))
    for dev in devices:
        print(
            f"{dev.get('id', '')[:33]:<35} "
            f"{dev.get('name', '')[:28]:<30} "
            f"{dev.get('bus', '?'):<10} "
            f"{(dev.get('driver') or '(none)')[:18]:<20} "
            f"{dev.get('status', 'Unknown'):<12}"
        )


def print_device_detail(device: dict):
    if not device:
        print("Device not found")
        return

    if HAS_RICH:
        console = Console()
        console.print(f"\n[bold]{device.get('name', device.get('id', ''))}[/bold]")
        console.print(f"  ID:        {device.get('id', '')}")
        console.print(f"  Bus:       {device.get('bus', '?')}")
        console.print(f"  Bus ID:    {device.get('bus_id', '-')}")
        console.print(f"  Vendor:    {device.get('vendor', '-')}")
        console.print(f"  Model:     {device.get('model', '-')}")
        console.print(f"  Driver:    {device.get('driver', '(none)')}")
        console.print(f"  Status:    {device.get('status', 'Unknown')}")
        console.print(f"  Subsystem: {device.get('subsystem', '')}")
        console.print(f"  Path:      {device.get('path', '')}")
        console.print(f"  Removable: {'Yes' if device.get('removable') else 'No'}")
        console.print(f"  Parent:    {device.get('parent', '(none)')}")
        console.print(f"  Children:  {len(device.get('children', []))}")
        if device.get("properties"):
            console.print("\n  [bold]Properties:[/bold]")
            for key, val in sorted(device["properties"].items()):
                console.print(f"    {key} = {val}")
    else:
        print(f"\n{device.get('name', device.get('id', ''))}")
        print(f"  ID:        {device.get('id', '')}")
        print(f"  Bus:       {device.get('bus', '?')}")
        print(f"  Bus ID:    {device.get('bus_id', '-')}")
        print(f"  Vendor:    {device.get('vendor', '-')}")
        print(f"  Model:     {device.get('model', '-')}")
        print(f"  Driver:    {device.get('driver', '(none)')}")
        print(f"  Status:    {device.get('status', 'Unknown')}")
        print(f"  Subsystem: {device.get('subsystem', '')}")
        print(f"  Path:      {device.get('path', '')}")


def print_json(data):
    print(json.dumps(data, indent=2))


def print_watch_event(event_type: str, data: str):
    if HAS_RICH:
        console = Console()
        color = {"add": "green", "remove": "red", "change": "yellow"}.get(event_type, "white")
        console.print(f"[{color}][{event_type.upper()}][/{color}] {data}")
    else:
        print(f"[{event_type.upper()}] {data}")
