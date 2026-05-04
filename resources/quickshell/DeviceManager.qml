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
    property var devices: []
    property bool loading: false
    property string deviceCount: ""

    Timer {
        id: refreshTimer
        interval: 5000; repeat: true
        running: dmgrRoot.drawerOpen
        onTriggered: loadDevices()
    }

    function loadDevices() {
        if (loading) return
        loading = true
        let process = new Process()
        process.command = ["busctl", "--user", "call",
            "org.dmgr.DeviceManager", "/org/dmgr/DeviceManager",
            "org.dmgr.DeviceManager", "GetAllDevices"]
        process.onFinished = function(code, stdout, stderr) {
            loading = false
            if (code !== 0) {
                deviceCount = "⚫"
                return
            }
            let raw = stdout.trim()
            if (raw.startsWith('s ')) {
                raw = raw.substring(2).replace(/\\"/g, '"')
                try {
                    let data = JSON.parse(raw)
                    devices = data
                    let online = data.filter(d => d.status === "Online").length
                    deviceCount = data.length + " devs (" + online + " online)"
                } catch(e) {
                    deviceCount = "Error"
                }
            }
        }
        process.start()
    }

    Component.onCompleted: loadDevices()

    Connections {
        target: dmgrRoot
        function onDrawerOpenChanged() { if (drawerOpen) loadDevices() }
    }

    Rectangle {
        id: iconContainer
        anchors.fill: parent
        radius: 18
        color: dmgrRoot.drawerOpen ? "#4a6fa5" : (iconMouse.containsMouse ? "#e8dccb" : "transparent")

        Behavior on color { ColorAnimation { duration: 200 } }

        Text {
            anchors.centerIn: parent
            text: dmgrRoot.drawerOpen ? "🔌" : "🔧"
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
        height: parent.height
        color: "#f5ede4"
        radius: 16
        clip: true

        border { width: 1; color: "#e0d8cc" }

        Behavior on opacity { NumberAnimation { duration: 200 } }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 8

            Text {
                text: "Device Manager"
                font { pixelSize: 16; bold: true }
                color: "#4a3729"
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: "#e0d8cc" }

            BusyIndicator {
                Layout.alignment: Qt.AlignHCenter
                visible: dmgrRoot.loading
                running: dmgrRoot.loading
            }

            Text {
                text: dmgrRoot.deviceCount
                color: "#8b7355"
                font.pixelSize: 11
                visible: !dmgrRoot.loading
            }

            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true

                Column {
                    width: parent.width
                    spacing: 4

                    Repeater {
                        model: dmgrRoot.devices.slice(0, 20)

                        Rectangle {
                            width: parent.width
                            height: 36
                            color: mouseRow.containsMouse ? "#e8dccb" : "transparent"
                            radius: 8

                            RowLayout {
                                anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
                                spacing: 6

                                Rectangle {
                                    width: 8; height: 8; radius: 4
                                    color: {
                                        switch(modelData.status) {
                                            case "Online": return "#4caf50"
                                            case "Suspended": return "#ff9800"
                                            case "Offline": return "#f44336"
                                            case "Unbound": return "#9e9e9e"
                                            default: return "#ccc"
                                        }
                                    }
                                }

                                Column {
                                    Layout.fillWidth: true
                                    Text {
                                        text: modelData.name || modelData.id
                                        color: "#4a3729"
                                        font.pixelSize: 12
                                        elide: Text.ElideRight
                                        width: 200
                                    }
                                    Text {
                                        text: (modelData.driver || "(none)") + " · " + (modelData.bus || "?")
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
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: "#e0d8cc" }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Button {
                    text: "↻"
                    implicitHeight: 28
                    background: Rectangle {
                        color: parent.hovered ? "#e8dccb" : "transparent"
                        radius: 8
                    }
                    onClicked: dmgrRoot.loadDevices()
                }

                Button {
                    text: "Open Full Manager"
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
                        let process = new Process()
                        process.command = ["qml6", "/usr/share/dmgr/qml/dmgr-standalone.qml"]
                        process.start()
                    }
                }
            }

            Text {
                text: dmgrRoot.devices.length > 20 ? "+" + (dmgrRoot.devices.length - 20) + " more devices..." : ""
                color: "#8b7355"
                font.pixelSize: 10
                visible: dmgrRoot.devices.length > 20
            }
        }
    }
}
