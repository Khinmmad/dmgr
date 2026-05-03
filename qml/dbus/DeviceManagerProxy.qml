pragma Singleton
import QtQuick 6.0

QtObject {
    id: root

    property string serviceName: "org.dmgr.DeviceManager"
    property string objectPath: "/org/dmgr/DeviceManager"
    property string interfaceName: "org.dmgr.DeviceManager"

    property var devices: []
    property var selectedDevice: null
    property bool loading: false
    property string statusMessage: ""

    signal devicesLoaded(var deviceList)
    signal deviceUpdated(string devId)
    signal errorOccurred(string message)

    function callMethod(methodName, args) {
        if (typeof DBusConnection === "undefined") {
            console.warn("DBusConnection not available, using mock data")
            return null
        }

        var params = args || []
        return DBusConnection.sessionBus().call(
            serviceName, objectPath, interfaceName, methodName, params
        )
    }

    function loadAllDevices() {
        root.loading = true
        root.statusMessage = "Scanning devices..."
        try {
            var jsonStr = callMethod("GetAllDevices", [])
            if (jsonStr) {
                root.devices = JSON.parse(jsonStr)
                root.devicesLoaded(root.devices)
                root.statusMessage = root.devices.length + " devices found"
            }
        } catch (e) {
            root.statusMessage = "Error: " + e
            root.errorOccurred(e.toString())
        }
        root.loading = false
    }

    function selectDevice(devId) {
        try {
            var jsonStr = callMethod("GetDevice", [devId])
            if (jsonStr && jsonStr !== "{}") {
                root.selectedDevice = JSON.parse(jsonStr)
                root.deviceUpdated(devId)
            }
        } catch (e) {
            root.errorOccurred(e.toString())
        }
    }

    function getDevicesByBus(bus) {
        try {
            var jsonStr = callMethod("GetDevicesByBus", [bus])
            if (jsonStr) {
                return JSON.parse(jsonStr)
            }
        } catch (e) {
            root.errorOccurred(e.toString())
        }
        return []
    }

    function searchDevices(query) {
        try {
            var jsonStr = callMethod("GetDevicesByFilter", [query])
            if (jsonStr) {
                root.devices = JSON.parse(jsonStr)
                root.devicesLoaded(root.devices)
                root.statusMessage = root.devices.length + " results for '" + query + "'"
            }
        } catch (e) {
            root.errorOccurred(e.toString())
        }
    }

    function getAvailableDrivers(devId) {
        try {
            return callMethod("GetAvailableDrivers", [devId])
        } catch (e) {
            return []
        }
    }

    function bindDriver(devId, driver) {
        try {
            var ok = callMethod("BindDriver", [devId, driver])
            return ok === true
        } catch (e) {
            root.errorOccurred("Bind failed: " + e)
            return false
        }
    }

    function unbindDriver(devId) {
        try {
            var ok = callMethod("UnbindDriver", [devId])
            return ok === true
        } catch (e) {
            root.errorOccurred("Unbind failed: " + e)
            return false
        }
    }

    function setProperty(devId, attr, value) {
        try {
            var ok = callMethod("SetProperty", [devId, attr, value])
            return ok === true
        } catch (e) {
            root.errorOccurred("Set property failed: " + e)
            return false
        }
    }

    function refresh() {
        root.loading = true
        try {
            callMethod("Refresh", [])
            loadAllDevices()
        } catch (e) {
            root.errorOccurred(e.toString())
        }
    }

    Component.onCompleted: {
        loadAllDevices()
    }
}
