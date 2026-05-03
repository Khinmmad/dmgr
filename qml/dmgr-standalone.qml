import QtQuick 6.0
import QtQuick.Controls 6.0
import QtQuick.Layouts 6.0
import dmgr 1.0

ApplicationWindow {
    id: app
    title: "dmgr — Device Manager"
    width: 1100
    height: 700
    minimumWidth: 800
    minimumHeight: 500
    visible: true

    color: DmgrTheme.bgPrimary

    menuBar: MenuBar {
        Menu {
            title: "&File"
            Action {
                text: "&Refresh"
                shortcut: "F5"
                onTriggered: DeviceManagerProxy.refresh()
            }
            MenuSeparator {}
            Action {
                text: "&Quit"
                shortcut: "Ctrl+Q"
                onTriggered: Qt.quit()
            }
        }

        Menu {
            title: "&View"
            Action {
                text: "Show &All Devices"
                checkable: true
                checked: true
                onTriggered: DeviceManagerProxy.loadAllDevices()
            }
            MenuSeparator {}
            Action {
                text: "USB Devices"
                onTriggered: DeviceManagerProxy.getDevicesByBus("Usb")
            }
            Action {
                text: "PCI Devices"
                onTriggered: DeviceManagerProxy.getDevicesByBus("Pci")
            }
            Action {
                text: "Audio Devices"
                onTriggered: DeviceManagerProxy.getDevicesByBus("Audio")
            }
            Action {
                text: "Input Devices"
                onTriggered: DeviceManagerProxy.getDevicesByBus("Input")
            }
            Action {
                text: "Block Devices"
                onTriggered: DeviceManagerProxy.getDevicesByBus("Block")
            }
            Action {
                text: "Network Devices"
                onTriggered: DeviceManagerProxy.getDevicesByBus("Net")
            }
        }

        Menu {
            title: "&Help"
            Action {
                text: "&About"
                onTriggered: aboutDialog.open()
            }
        }
    }

    ColumnLayout {
        anchors {
            fill: parent
            margins: 0
        }
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            DeviceTree {
                id: deviceTree
                Layout.preferredWidth: 300
                Layout.fillHeight: true

                onDeviceSelected: function(devId) {
                    detailControls.deviceId = devId
                }
            }

            Rectangle {
                width: 1
                Layout.fillHeight: true
                color: DmgrTheme.border
            }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                RowLayout {
                    anchors.fill: parent
                    spacing: 0

                    DeviceDetail {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                    }

                    Rectangle {
                        width: 1
                        Layout.fillHeight: true
                        color: DmgrTheme.border
                    }

                    DeviceControls {
                        id: detailControls
                        Layout.preferredWidth: 260
                        Layout.fillHeight: true
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: DmgrTheme.border
        }

        Rectangle {
            Layout.fillWidth: true
            height: 28
            color: DmgrTheme.bgSecondary

            RowLayout {
                anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                spacing: 8

                Text {
                    text: DeviceManagerProxy.statusMessage || "Ready"
                    color: DmgrTheme.textMuted
                    font { pixelSize: DmgrTheme.fontSizeSmall; family: DmgrTheme.fontFamily }
                    Layout.fillWidth: true
                }

                Button {
                    text: "↻ Refresh (F5)"
                    implicitHeight: 22
                    background: Rectangle {
                        color: parent.hovered ? DmgrTheme.bgTertiary : "transparent"
                        radius: DmgrTheme.radiusSmall
                    }
                    contentItem: Text {
                        text: parent.text
                        color: DmgrTheme.textSecondary
                        font { pixelSize: DmgrTheme.fontSizeSmall; family: DmgrTheme.fontFamily }
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                    onClicked: DeviceManagerProxy.refresh()
                }
            }
        }
    }

    Dialog {
        id: aboutDialog
        title: "About dmgr"
        modal: true
        standardButtons: Dialog.Ok

        contentItem: ColumnLayout {
            spacing: 12
            Text {
                text: "dmgr — Device Manager v1.0.0"
                font { bold: true; pixelSize: 16; family: DmgrTheme.fontFamily }
                color: DmgrTheme.textPrimary
            }
            Text {
                text: "Administrador de dispositivos al estilo Windows para Arch Linux.\n\nPowered by Rust, QML, and QuickShell.\n\nhttps://github.com/isra/dmgr"
                font { pixelSize: 13; family: DmgrTheme.fontFamily }
                color: DmgrTheme.textSecondary
                wrapMode: Text.WordWrap
                Layout.preferredWidth: 350
            }
        }
    }
}
