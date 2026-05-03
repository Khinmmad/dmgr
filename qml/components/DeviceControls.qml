import QtQuick 6.0
import QtQuick.Controls 6.0
import QtQuick.Layouts 6.0
import dmgr 1.0

Rectangle {
    id: root
    color: DmgrTheme.bgPrimary
    implicitWidth: 280

    property string deviceId: ""
    property var device: null

    ColumnLayout {
        anchors {
            fill: parent
            margins: DmgrTheme.spacingLarge
        }
        spacing: DmgrTheme.spacing

        Text {
            text: "Actions"
            color: DmgrTheme.accent
            font { pixelSize: DmgrTheme.fontSizeLarge; bold: true }
            font.family: DmgrTheme.fontFamily
        }

        Rectangle { Layout.fillWidth: true; height: 1; color: DmgrTheme.border }

        Text {
            text: root.device ? "Driver: " + (root.device.driver || "(none)") : ""
            color: DmgrTheme.textSecondary
            font { pixelSize: DmgrTheme.fontSize }
            font.family: DmgrTheme.fontFamily
            Layout.fillWidth: true
            elide: Text.ElideRight
        }

        Button {
            text: "Unbind Driver"
            Layout.fillWidth: true
            enabled: root.device && root.device.driver !== null

            background: Rectangle {
                color: parent.enabled ? (parent.hovered ? "#d32f2f" : DmgrTheme.error) : "#444"
                radius: DmgrTheme.radiusSmall
            }
            contentItem: Text {
                text: parent.text
                color: "white"
                font { pixelSize: DmgrTheme.fontSize; bold: true }
                font.family: DmgrTheme.fontFamily
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }

            onClicked: {
                if (root.deviceId) {
                    var ok = DeviceManagerProxy.unbindDriver(root.deviceId)
                    if (ok) DeviceManagerProxy.selectDevice(root.deviceId)
                }
            }
        }

        Text {
            text: "Available Drivers"
            color: DmgrTheme.textSecondary
            font { pixelSize: DmgrTheme.fontSizeSmall; bold: true }
            font.family: DmgrTheme.fontFamily
            Layout.topMargin: 8
        }

        ListView {
            id: driverList
            Layout.fillWidth: true
            Layout.preferredHeight: 120
            clip: true
            model: root.device ? DeviceManagerProxy.getAvailableDrivers(root.deviceId) : []
            ScrollBar.vertical: ScrollBar {}

            delegate: Rectangle {
                width: driverList.width
                height: 30
                color: mouseDrv.containsMouse ? DmgrTheme.bgTertiary : "transparent"

                RowLayout {
                    anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
                    Text {
                        text: modelData
                        color: DmgrTheme.textPrimary
                        font { pixelSize: DmgrTheme.fontSizeSmall; family: DmgrTheme.fontMono }
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }
                    Button {
                        text: "Bind"
                        implicitHeight: 24
                        background: Rectangle {
                            color: parent.hovered ? DmgrTheme.accentHover : DmgrTheme.accent
                            radius: DmgrTheme.radiusSmall
                        }
                        contentItem: Text {
                            text: parent.text
                            color: "white"
                            font { pixelSize: 10; bold: true }
                            font.family: DmgrTheme.fontFamily
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
                        onClicked: {
                            var ok = DeviceManagerProxy.bindDriver(root.deviceId, modelData)
                            if (ok) DeviceManagerProxy.selectDevice(root.deviceId)
                        }
                    }
                }

                MouseArea {
                    id: mouseDrv
                    anchors.fill: parent
                    hoverEnabled: true
                }
            }
        }

        Rectangle { Layout.fillWidth: true; height: 1; color: DmgrTheme.border }

        Text {
            text: "Editable Properties"
            color: DmgrTheme.textSecondary
            font { pixelSize: DmgrTheme.fontSizeSmall; bold: true }
            font.family: DmgrTheme.fontFamily
        }

        ListView {
            id: editableList
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.device ? (root.device.editable_properties || []) : []
            ScrollBar.vertical: ScrollBar {}

            delegate: PropertyEditor {
                width: editableList.width
                deviceId: root.deviceId
                propertyName: modelData
                currentValue: root.device ? (root.device.properties[modelData] || "") : ""
            }
        }
    }

    Timer {
        id: refreshTimer
        interval: 500
        onTriggered: {
            if (root.deviceId) {
                DeviceManagerProxy.selectDevice(root.deviceId)
            }
        }
    }

    Connections {
        target: DeviceManagerProxy
        function onDeviceUpdated(devId) {
            if (devId === root.deviceId) {
                root.device = DeviceManagerProxy.selectedDevice
            }
        }
    }

    onDeviceIdChanged: {
        root.device = DeviceManagerProxy.selectedDevice
        if (!root.device && deviceId !== "") {
            DeviceManagerProxy.selectDevice(deviceId)
        }
    }
}
