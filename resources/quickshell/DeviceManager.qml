import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

Item {
    id: dmgrRoot
    implicitWidth: 36
    implicitHeight: 36

    property bool drawerOpen: false
    property bool loading: false
    property string statusText: "Click ↻ to load"
    property int deviceCount: 0
    property int onlineCount: 0

    ListModel { id: deviceModel }

    Process {
        id: busctlProcess
        command: ["/usr/bin/busctl", "--user", "call",
            "org.dmgr.DeviceManager", "/org/dmgr/DeviceManager",
            "org.dmgr.DeviceManager", "GetAllDevices"]
        running: false

        onFinished: function(code, stdout, stderr) {
            dmgrRoot.loading = false
            console.log("dmgr: busctl finished code=" + code + " stdout_len=" + stdout.length)
            if (code !== 0) {
                dmgrRoot.statusText = "Error: code " + code
                if (stderr.length > 0) dmgrRoot.statusText += " - " + stderr.trim().substring(0, 60)
                return
            }
            var raw = stdout.trim()
            if (raw.startsWith('s "') || raw.startsWith('s"'))
                raw = raw.substring(raw.indexOf('"') + 1)
            else if (raw.startsWith('s '))
                raw = raw.substring(2)
            if (raw.endsWith('"'))
                raw = raw.substring(0, raw.length - 1)

            raw = raw.replace(/\\"/g, '"')

            try {
                var data = JSON.parse(raw)
                deviceModel.clear()
                var online = 0
                var maxShow = 20
                for (var i = 0; i < data.length && i < maxShow; i++) {
                    var d = data[i]
                    deviceModel.append({
                        name: d.name || d.id,
                        bus: d.bus || "?",
                        driver: d.driver || "(none)",
                        status: d.status || "?",
                        id: d.id
                    })
                    if (d.status === "Online") online++
                }
                dmgrRoot.deviceCount = data.length
                dmgrRoot.onlineCount = online
                dmgrRoot.statusText = data.length + " devices (" + online + " online)"
                if (data.length > maxShow) dmgrRoot.statusText += " — showing first " + maxShow
            } catch(e) {
                dmgrRoot.statusText = "JSON error: " + e.toString().substring(0, 50)
                console.log("dmgr: parse error " + e + " raw=" + raw.substring(0, 100))
            }
        }
    }

    function loadDevices() {
        if (dmgrRoot.loading || busctlProcess.running) return
        console.log("dmgr: loading devices...")
        dmgrRoot.loading = true
        dmgrRoot.statusText = "Loading..."
        busctlProcess.running = true
    }

    Component.onCompleted: loadDevices()

    onDrawerOpenChanged: { if (drawerOpen) loadDevices() }

    Rectangle {
        id: iconContainer
        anchors.fill: parent
        radius: 18
        color: dmgrRoot.drawerOpen ? "#4a6fa5" : (iconMouse.containsMouse ? "#e8dccb" : "transparent")
        Behavior on color { ColorAnimation { duration: 200 } }

        Text {
            anchors.centerIn: parent
            text: "🔧"
            font.pixelSize: 18
        }

        MouseArea {
            id: iconMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: dmgrRoot.drawerOpen = !dmgrRoot.drawerOpen
        }
    }

    Rectangle {
        id: drawer
        visible: dmgrRoot.drawerOpen
        anchors {
            top: parent.top
            bottom: parent.bottom
            left: parent.right
            leftMargin: 12
        }
        width: 320
        color: "#f5ede4"
        radius: 16
        clip: true
        border { width: 1; color: "#e0d8cc" }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 8

            Text {
                text: "Device Manager"
                font { pixelSize: 16; bold: true }
                color: "#4a3729"
                Layout.fillWidth: true
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: "#e0d8cc" }

            BusyIndicator {
                Layout.alignment: Qt.AlignHCenter
                visible: dmgrRoot.loading || busctlProcess.running
                running: dmgrRoot.loading || busctlProcess.running
            }

            Text {
                text: dmgrRoot.statusText
                color: "#8b7355"
                font.pixelSize: 11
                visible: !(dmgrRoot.loading || busctlProcess.running)
                Layout.fillWidth: true
            }

            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true

                ListView {
                    id: deviceListView
                    width: parent.width
                    model: deviceModel
                    spacing: 4
                    clip: true
                    interactive: true

                    delegate: Rectangle {
                        width: deviceListView.width
                        height: 36
                        color: mouseRow.containsMouse ? "#e8dccb" : "transparent"
                        radius: 8

                        RowLayout {
                            anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
                            spacing: 6

                            Rectangle {
                                width: 8; height: 8; radius: 4
                                color: {
                                    if (status === "Online") return "#4caf50"
                                    if (status === "Suspended") return "#ff9800"
                                    if (status === "Offline") return "#f44336"
                                    if (status === "Unbound") return "#9e9e9e"
                                    return "#ccc"
                                }
                            }

                            Column {
                                Layout.fillWidth: true
                                Text {
                                    text: name
                                    color: "#4a3729"
                                    font.pixelSize: 12
                                    elide: Text.ElideRight
                                    width: 200
                                }
                                Text {
                                    text: driver + " · " + bus
                                    color: "#8b7355"
                                    font.pixelSize: 9
                                    font.family: "monospace"
                                }
                            }
                        }

                        MouseArea {
                            id: mouseRow
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                        }
                    }
                }
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: "#e0d8cc" }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Button {
                    text: "↻ Refresh"
                    implicitHeight: 28
                    background: Rectangle {
                        color: parent.hovered ? "#e8dccb" : "transparent"
                        radius: 8
                    }
                    contentItem: Text {
                        text: parent.text
                        color: "#4a3729"
                        font.pixelSize: 11
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                    onClicked: dmgrRoot.loadDevices()
                }

                Button {
                    text: "Full Manager"
                    Layout.fillWidth: true
                    implicitHeight: 28
                    background: Rectangle {
                        color: parent.hovered ? "#5a8dee" : "#4a6fa5"
                        radius: 8
                    }
                    contentItem: Text {
                        text: parent.text
                        color: "white"
                        font.pixelSize: 11
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                    onClicked: {
                        var p = Qt.createQmlObject('import Quickshell.Io; Process { command: ["/usr/bin/qml6", "/usr/share/dmgr/qml/dmgr-standalone.qml"] }', dmgrRoot)
                        p.start()
                    }
                }
            }
        }
    }
}
