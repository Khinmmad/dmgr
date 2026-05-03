import QtQuick 6.0
import QtQuick.Controls 6.0
import QtQuick.Layouts 6.0
import dmgr 1.0

Rectangle {
    id: root
    color: DmgrTheme.bgSecondary
    height: 44

    signal search(string query)

    RowLayout {
        anchors {
            fill: parent
            leftMargin: 12
            rightMargin: 12
        }
        spacing: 8

        Image {
            source: "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='%23a0a0b0'><path d='M15.5 14h-.79l-.28-.27A6.471 6.471 0 0 0 16 9.5 6.5 6.5 0 1 0 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z'/></svg>"
            width: 18
            height: 18
            opacity: 0.6
        }

        TextField {
            id: searchField
            Layout.fillWidth: true
            placeholderText: "Search devices..."
            placeholderTextColor: DmgrTheme.textMuted
            color: DmgrTheme.textPrimary
            font { pixelSize: DmgrTheme.fontSize; family: DmgrTheme.fontFamily }

            background: Rectangle {
                color: "transparent"
            }

            onTextChanged: {
                searchTimer.restart()
            }
        }

        Text {
            text: DeviceManagerProxy.devices.length + " devices"
            color: DmgrTheme.textMuted
            font { pixelSize: DmgrTheme.fontSizeSmall; family: DmgrTheme.fontFamily }
            visible: !DeviceManagerProxy.loading
        }

        BusyIndicator {
            visible: DeviceManagerProxy.loading
            running: DeviceManagerProxy.loading
            implicitWidth: 16
            implicitHeight: 16
        }
    }

    Timer {
        id: searchTimer
        interval: 300
        onTriggered: {
            root.search(searchField.text)
        }
    }
}
