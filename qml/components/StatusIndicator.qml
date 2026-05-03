import QtQuick 6.0
import dmgr 1.0

Rectangle {
    id: root
    width: 12
    height: 12
    radius: 6

    property string status: "Unknown"

    color: {
        switch (status) {
            case "Online": return DmgrTheme.online
            case "Offline": return DmgrTheme.offline
            case "Suspended": return DmgrTheme.suspended
            case "Unbound": return DmgrTheme.unbound
            case "Error": return DmgrTheme.error
            default: return DmgrTheme.unbound
        }
    }

    border {
        width: 1
        color: Qt.rgba(1, 1, 1, 0.2)
    }
}
