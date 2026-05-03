#!/usr/bin/env python3
"""dmgr - Device Manager CLI."""

import sys
import time

from dmgr.client import DMgrClient
from dmgr.formatters import (
    print_devices_table,
    print_device_detail,
    print_json,
    print_watch_event,
)


def cmd_list(args):
    client = DMgrClient()
    try:
        if args.bus:
            devices = client.get_devices_by_bus(args.bus)
            title = f"Devices — {args.bus}"
        elif args.search:
            devices = client.search_devices(args.search)
            title = f"Search: {args.search}"
        else:
            devices = client.get_all_devices()
            title = "All Devices"

        if args.json:
            print_json(devices)
        else:
            print_devices_table(devices, title)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def cmd_info(args):
    client = DMgrClient()
    try:
        device = client.get_device(args.dev_id)
        if args.json:
            print_json(device)
        else:
            print_device_detail(device)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def cmd_search(args):
    client = DMgrClient()
    try:
        devices = client.search_devices(args.query)
        if args.json:
            print_json(devices)
        else:
            print_devices_table(devices, f"Search: {args.query}")
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def cmd_bind(args):
    client = DMgrClient()
    try:
        ok = client.bind_driver(args.dev_id, args.driver)
        if ok:
            print(f"Bound {args.dev_id} to {args.driver}")
        else:
            print("Bind failed (check permissions)", file=sys.stderr)
            sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def cmd_unbind(args):
    client = DMgrClient()
    try:
        ok = client.unbind_driver(args.dev_id)
        if ok:
            print(f"Unbound {args.dev_id}")
        else:
            print("Unbind failed (check permissions)", file=sys.stderr)
            sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def cmd_property_get(args):
    client = DMgrClient()
    try:
        device = client.get_device(args.dev_id)
        if device and "properties" in device:
            value = device["properties"].get(args.attr, "(not found)")
            print(value)
        else:
            print("Device not found", file=sys.stderr)
            sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def cmd_property_set(args):
    client = DMgrClient()
    try:
        ok = client.set_property(args.dev_id, args.attr, args.value)
        if ok:
            print(f"Set {args.attr} = {args.value} on {args.dev_id}")
        else:
            print("Failed to set property", file=sys.stderr)
            sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def cmd_watch(args):
    client = DMgrClient()
    print("Watching device events... (Ctrl+C to stop)")
    try:
        while True:
            devices = client.get_all_devices()
            print(f"\rDevices: {len(devices)}", end="", flush=True)
            time.sleep(2)
    except KeyboardInterrupt:
        print("\nStopped.")


def cmd_refresh(args):
    client = DMgrClient()
    try:
        count = client.refresh()
        print(f"Refreshed: {count} devices")
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def cmd_drivers(args):
    client = DMgrClient()
    try:
        drivers = client.get_available_drivers(args.dev_id)
        for d in drivers:
            print(d)
        if not drivers:
            print("No drivers found")
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def main():
    import argparse

    parser = argparse.ArgumentParser(
        prog="dmgr",
        description="Device Manager CLI for Arch Linux",
    )
    sub = parser.add_subparsers(dest="command")

    p_list = sub.add_parser("list", help="List devices")
    p_list.add_argument("--bus", choices=["pci", "usb", "audio", "input", "block", "drm", "net", "hid", "tty", "power"], help="Filter by bus")
    p_list.add_argument("--search", help="Search by query")
    p_list.add_argument("--json", action="store_true", help="JSON output")

    p_info = sub.add_parser("info", help="Device details")
    p_info.add_argument("dev_id", help="Device ID")
    p_info.add_argument("--json", action="store_true", help="JSON output")

    p_search = sub.add_parser("search", help="Search devices")
    p_search.add_argument("query", help="Search query")
    p_search.add_argument("--json", action="store_true", help="JSON output")

    p_bind = sub.add_parser("bind", help="Bind driver to device")
    p_bind.add_argument("dev_id", help="Device ID")
    p_bind.add_argument("driver", help="Driver name")

    p_unbind = sub.add_parser("unbind", help="Unbind driver from device")
    p_unbind.add_argument("dev_id", help="Device ID")

    p_prop = sub.add_parser("property", help="Get/set device property")
    p_prop_sub = p_prop.add_subparsers(dest="prop_cmd")
    p_get = p_prop_sub.add_parser("get", help="Get property")
    p_get.add_argument("dev_id", help="Device ID")
    p_get.add_argument("attr", help="Property name (e.g. power/control)")
    p_set = p_prop_sub.add_parser("set", help="Set property")
    p_set.add_argument("dev_id", help="Device ID")
    p_set.add_argument("attr", help="Property name")
    p_set.add_argument("value", help="New value")

    p_watch = sub.add_parser("watch", help="Watch device events (polling)")

    p_refresh = sub.add_parser("refresh", help="Refresh device scan")

    p_drivers = sub.add_parser("drivers", help="List available drivers for device")
    p_drivers.add_argument("dev_id", help="Device ID")

    args = parser.parse_args()

    if args.command is None:
        parser.print_help()
        sys.exit(0)

    dispatcher = {
        "list": cmd_list,
        "info": cmd_info,
        "search": cmd_search,
        "bind": cmd_bind,
        "unbind": cmd_unbind,
        "watch": cmd_watch,
        "refresh": cmd_refresh,
        "drivers": cmd_drivers,
    }

    if args.command == "property":
        if args.prop_cmd == "get":
            cmd_property_get(args)
        elif args.prop_cmd == "set":
            cmd_property_set(args)
        else:
            parser.parse_args(["property", "--help"])
    elif args.command in dispatcher:
        dispatcher[args.command](args)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
