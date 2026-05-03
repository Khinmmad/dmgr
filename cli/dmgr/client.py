"""DBus client for dmgr daemon using dasbus."""

import json
from dasbus.connection import SessionMessageBus

SERVICE = "org.dmgr.DeviceManager"
OBJECT = "/org/dmgr/DeviceManager"
INTERFACE = "org.dmgr.DeviceManager"


class DMgrClient:
    """Client for the dmgr daemon via DBus."""

    def __init__(self):
        self._bus = SessionMessageBus()
        self._proxy = self._bus.get_proxy(SERVICE, OBJECT)

    def call(self, method: str, *args):
        return self._proxy.call(method, *args)

    def get_all_devices(self) -> list:
        result = self.call("GetAllDevices")
        if result:
            return json.loads(result) if isinstance(result, str) else result
        return []

    def get_device(self, dev_id: str) -> dict | None:
        result = self.call("GetDevice", dev_id)
        if result and result != "{}":
            return json.loads(result) if isinstance(result, str) else result
        return None

    def get_devices_by_bus(self, bus: str) -> list:
        result = self.call("GetDevicesByBus", bus)
        if result:
            return json.loads(result) if isinstance(result, str) else result
        return []

    def search_devices(self, query: str) -> list:
        result = self.call("GetDevicesByFilter", query)
        if result:
            return json.loads(result) if isinstance(result, str) else result
        return []

    def get_available_drivers(self, dev_id: str) -> list:
        return self.call("GetAvailableDrivers", dev_id) or []

    def bind_driver(self, dev_id: str, driver: str) -> bool:
        return self.call("BindDriver", dev_id, driver)

    def unbind_driver(self, dev_id: str) -> bool:
        return self.call("UnbindDriver", dev_id)

    def set_property(self, dev_id: str, attr: str, value: str) -> bool:
        return self.call("SetProperty", dev_id, attr, value)

    def refresh(self) -> int:
        return self.call("Refresh") or 0
