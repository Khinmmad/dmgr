import QtQuick 6.0
import QtQuick.Controls 6.0
import QtQuick.Layouts 6.0
import dmgr 1.0

Rectangle {
    id: root
    color: DmgrTheme.bgPrimary
    implicitWidth: 400

    property var device: DeviceManagerProxy.selectedDevice

    ColumnLayout {
        anchors {
            fill: parent
            margins: DmgrTheme.spacingLarge
        }
        spacing: DmgrTheme.spacingLarge

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: DmgrTheme.border
            visible: root.device === null
        }

        Text {
            text: "No device selected"
            color: DmgrTheme.textMuted
            font { pixelSize: DmgrTheme.fontSizeLarge; bold: true }
            font.family: DmgrTheme.fontFamily
            visible: root.device === null
            Layout.alignment: Qt.AlignCenter
        }

        ColumnLayout {
            visible: root.device !== null
            spacing: DmgrTheme.spacingLarge
            Layout.fillWidth: true

            Text {
                text: root.device ? (root.device.name || root.device.id) : ""
                color: DmgrTheme.textPrimary
                font { pixelSize: DmgrTheme.fontSizeTitle; bold: true }
                font.family: DmgrTheme.fontFamily
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            RowLayout {
                spacing: 12
                StatusIndicator { status: root.device ? root.device.status : "Unknown" }
                Text {
                    text: root.device ? root.device.status : ""
                    color: DmgrTheme.textSecondary
                    font { pixelSize: DmgrTheme.fontSize }
                    font.family: DmgrTheme.fontFamily
                }
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: DmgrTheme.border }

            GridLayout {
                columns: 2
                rowSpacing: 8
                columnSpacing: 16
                Layout.fillWidth: true

                detailLabel("Bus:")
                detailValue(root.device ? root.device.bus : "")

                detailLabel("Bus ID:")
                detailValue(root.device ? (root.device.bus_id || "-") : "")

                detailLabel("Vendor:")
                detailValue(root.device ? (root.device.vendor || "-") : "")

                detailLabel("Vendor ID:")
                detailValueMono(root.device ? (root.device.vendor_id || "-") : "")

                detailLabel("Model:")
                detailValue(root.device ? (root.device.model || "-") : "")

                detailLabel("Model ID:")
                detailValueMono(root.device ? (root.device.model_id || "-") : "")

                detailLabel("Driver:")
                detailValue(root.device ? (root.device.driver || "(none)") : "")

                detailLabel("Subsystem:")
                detailValue(root.device ? (root.device.subsystem || "") : "")

                detailLabel("Path:")
                detailValueMono(root.device ? (root.device.path || "") : "")

                detailLabel("Removable:")
                detailValue(root.device ? (root.device.removable ? "Yes" : "No") : "")

                detailLabel("Parent:")
                detailValueMono(root.device ? (root.device.parent || "(none)") : "")
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: DmgrTheme.border }

            Text {
                text: "Properties"
                color: DmgrTheme.accent
                font { pixelSize: DmgrTheme.fontSizeLarge; bold: true }
                font.family: DmgrTheme.fontFamily
            }

            Flickable {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                contentHeight: propertiesColumn.implicitHeight
                ScrollBar.vertical: ScrollBar {}

                Column {
                    id: propertiesColumn
                    width: parent.width
                    spacing: 4

                    Repeater {
                        model: root.device ? Object.keys(root.device.properties || {}) : []

                        RowLayout {
                            width: parent.width
                            spacing: 8

                            Text {
                                text: modelData
                                color: DmgrTheme.textSecondary
                                font { pixelSize: DmgrTheme.fontSizeSmall; bold: true }
                                font.family: DmgrTheme.fontMono
                                Layout.preferredWidth: 160
                                elide: Text.ElideRight
                            }

                            Text {
                                text: root.device ? (root.device.properties[modelData] || "") : ""
                                color: DmgrTheme.textPrimary
                                font { pixelSize: DmgrTheme.fontSizeSmall }
                                font.family: DmgrTheme.fontMono
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                            }
                        }
                    }
                }
            }
        }
    }

    Component {
        id: labelComp
        Text {
            color: DmgrTheme.textSecondary
            font { pixelSize: DmgrTheme.fontSize }
            font.family: DmgrTheme.fontFamily
            text: ""
        }
    }

    Component {
        id: valueComp
        Text {
            color: DmgrTheme.textPrimary
            font { pixelSize: DmgrTheme.fontSize }
            font.family: DmgrTheme.fontFamily
            text: ""
            elide: Text.ElideRight
            Layout.fillWidth: true
        }
    }

    Component {
        id: valueMonoComp
        Text {
            color: DmgrTheme.textPrimary
            font { pixelSize: DmgrTheme.fontSize; family: DmgrTheme.fontMono }
            text: ""
            elide: Text.ElideMiddle
            Layout.fillWidth: true
        }
    }

    function detailLabel(t) { return Qt.createQmlObject('import QtQuick 6.0; Text { color: "' + DmgrTheme.textSecondary + '"; font { pixelSize: ' + DmgrTheme.fontSize + '; family: "' + DmgrTheme.fontFamily + '" } text: "' + t + '" }', root, "label") }
    function detailValue(t)  { return Qt.createQmlObject('import QtQuick 6.0; import QtQuick.Layouts 6.0; Text { color: "' + DmgrTheme.textPrimary + '"; font { pixelSize: ' + DmgrTheme.fontSize + '; family: "' + DmgrTheme.fontFamily + '" } text: "' + t + '"; elide: Text.ElideRight; Layout.fillWidth: true }', root, "value") }
    function detailValueMono(t) { return Qt.createQmlObject('import QtQuick 6.0; import QtQuick.Layouts 6.0; Text { color: "' + DmgrTheme.textPrimary + '"; font { pixelSize: ' + DmgrTheme.fontSize + '; family: "' + DmgrTheme.fontMono + '" } text: "' + t + '"; elide: Text.ElideMiddle; Layout.fillWidth: true }', root, "valuemono") }
}
