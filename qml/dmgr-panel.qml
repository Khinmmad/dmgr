import QtQuick 6.0
import QtQuick.Controls 6.0
import QtQuick.Layouts 6.0
import dmgr 1.0

QuickShell.Panel {
    id: panel
    title: "Device Manager"
    icon: "icons/dmgr.svg"

    ColumnLayout {
        anchors {
            fill: parent
            margins: 0
        }
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: DmgrTheme.border
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            DeviceTree {
                id: deviceTree
                Layout.preferredWidth: 280
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
                    text: "↻ Refresh"
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
}
