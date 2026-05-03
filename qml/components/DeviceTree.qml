import QtQuick 6.0
import QtQuick.Controls 6.0
import dmgr 1.0

Rectangle {
    id: root
    color: DmgrTheme.bgPrimary
    implicitWidth: 280

    property var currentDeviceId: ""
    signal deviceSelected(string devId)

    Column {
        anchors.fill: parent

        SearchBar {
            width: parent.width
            onSearch: function(query) {
                if (query.length > 1) {
                    DeviceManagerProxy.searchDevices(query)
                } else {
                    DeviceManagerProxy.loadAllDevices()
                }
            }
        }

        Rectangle {
            width: parent.width
            height: 1
            color: DmgrTheme.border
        }

        ListView {
            id: deviceList
            width: parent.width
            height: parent.height - 50
            clip: true
            model: deviceModel
            delegate: deviceDelegate
            ScrollBar.vertical: ScrollBar {}
        }
    }

    ListModel {
        id: deviceModel

        function rebuild(devices) {
            clear()
            if (!devices || devices.length === 0) return

            var buses = {}
            for (var i = 0; i < devices.length; i++) {
                var d = devices[i]
                var busName = d.bus || "Unknown"
                if (!buses[busName]) buses[busName] = []
                buses[busName].push(d)
            }

            var busOrder = ["Pci", "Usb", "Audio", "Input", "Block", "Drm", "Net", "Hid", "Tty", "Power"]
            for (var b = 0; b < busOrder.length; b++) {
                var bus = busOrder[b]
                if (buses[bus]) {
                    append({ type: "header", name: bus, count: buses[bus].length })
                    for (var d = 0; d < buses[bus].length; d++) {
                        append({ type: "device", data: buses[bus][d] })
                    }
                }
            }

            for (var other in buses) {
                if (busOrder.indexOf(other) === -1) {
                    append({ type: "header", name: other, count: buses[other].length })
                    for (var od = 0; od < buses[other].length; od++) {
                        append({ type: "device", data: buses[other][od] })
                    }
                }
            }
        }
    }

    Component {
        id: deviceDelegate

        Loader {
            width: deviceList.width
            sourceComponent: model.type === "header" ? headerComponent : deviceComponent
        }
    }

    Component {
        id: headerComponent

        Rectangle {
            width: parent.width
            height: 32
            color: DmgrTheme.bgSecondary

            Row {
                anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter }
                spacing: 8

                Text {
                    text: model.name
                    color: DmgrTheme.accent
                    font { bold: true; pixelSize: DmgrTheme.fontSize }
                    font.family: DmgrTheme.fontFamily
                }

                Text {
                    text: "(" + model.count + ")"
                    color: DmgrTheme.textMuted
                    font { pixelSize: DmgrTheme.fontSizeSmall }
                    font.family: DmgrTheme.fontFamily
                }
            }
        }
    }

    Component {
        id: deviceComponent

        Rectangle {
            width: parent.width
            height: 40
            color: {
                if (root.currentDeviceId === model.data.id) return DmgrTheme.bgTertiary
                if (mouseArea.containsMouse) return "#1a2a4a"
                return "transparent"
            }

            MouseArea {
                id: mouseArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor

                onClicked: {
                    root.currentDeviceId = model.data.id
                    DeviceManagerProxy.selectDevice(model.data.id)
                    root.deviceSelected(model.data.id)
                }
            }

            Row {
                anchors { left: parent.left; leftMargin: 20; right: parent.right; rightMargin: 12; verticalCenter: parent.verticalCenter }
                spacing: 8

                StatusIndicator {
                    status: model.data.status || "Unknown"
                    anchors.verticalCenter: parent.verticalCenter
                }

                Column {
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width - 40

                    Text {
                        text: model.data.name || model.data.id
                        color: DmgrTheme.textPrimary
                        font { pixelSize: DmgrTheme.fontSize }
                        font.family: DmgrTheme.fontFamily
                        elide: Text.ElideRight
                        width: parent.width
                    }

                    Text {
                        text: {
                            var parts = []
                            if (model.data.driver) parts.push(model.data.driver)
                            if (model.data.bus_id) parts.push(model.data.bus_id)
                            parts.join(" · ")
                        }
                        color: DmgrTheme.textMuted
                        font { pixelSize: DmgrTheme.fontSizeSmall }
                        font.family: DmgrTheme.fontMono
                        elide: Text.ElideRight
                        width: parent.width
                    }
                }
            }
        }
    }

    Connections {
        target: DeviceManagerProxy
        function onDevicesLoaded(devices) {
            deviceModel.rebuild(devices)
        }
    }
}
