import QtQuick 6.0
import QtQuick.Controls 6.0
import QtQuick.Layouts 6.0
import dmgr 1.0

Rectangle {
    id: root
    height: 36

    property string deviceId: ""
    property string propertyName: ""
    property string currentValue: ""

    RowLayout {
        anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
        spacing: 8

        Text {
            text: root.propertyName
            color: DmgrTheme.textMuted
            font { pixelSize: DmgrTheme.fontSizeSmall; family: DmgrTheme.fontMono }
            Layout.preferredWidth: 140
            elide: Text.ElideRight
        }

        TextField {
            id: valueField
            text: root.currentValue
            color: DmgrTheme.textPrimary
            font { pixelSize: DmgrTheme.fontSizeSmall; family: DmgrTheme.fontMono }
            Layout.fillWidth: true
            placeholderText: "value"

            background: Rectangle {
                color: DmgrTheme.bgSecondary
                radius: DmgrTheme.radiusSmall
                border.color: valueField.activeFocus ? DmgrTheme.accent : DmgrTheme.border
                border.width: 1
            }

            onEditingFinished: {
                if (text !== root.currentValue && root.deviceId && root.propertyName) {
                    var ok = DeviceManagerProxy.setProperty(root.deviceId, root.propertyName, text)
                    if (!ok) {
                        text = root.currentValue
                    }
                }
            }
        }

        Button {
            text: "Set"
            implicitHeight: 28
            enabled: valueField.text !== root.currentValue
            background: Rectangle {
                color: parent.enabled ? (parent.hovered ? DmgrTheme.accentHover : DmgrTheme.accent) : "#444"
                radius: DmgrTheme.radiusSmall
            }
            contentItem: Text {
                text: parent.text
                color: "white"
                font { pixelSize: 10; bold: true; family: DmgrTheme.fontFamily }
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
            onClicked: {
                if (root.deviceId && root.propertyName) {
                    var ok = DeviceManagerProxy.setProperty(root.deviceId, root.propertyName, valueField.text)
                }
            }
        }
    }
}
