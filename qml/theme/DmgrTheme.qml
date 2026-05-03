pragma Singleton
import QtQuick 6.0

QtObject {
    readonly property color bgPrimary: "#1a1a2e"
    readonly property color bgSecondary: "#16213e"
    readonly property color bgTertiary: "#0f3460"
    readonly property color accent: "#5a8dee"
    readonly property color accentHover: "#7ba3f5"
    readonly property color textPrimary: "#eaeaea"
    readonly property color textSecondary: "#a0a0b0"
    readonly property color textMuted: "#606070"
    readonly property color border: "#2a2a4a"
    readonly property color success: "#4caf50"
    readonly property color warning: "#ff9800"
    readonly property color error: "#f44336"
    readonly property color online: "#4caf50"
    readonly property color offline: "#f44336"
    readonly property color suspended: "#ff9800"
    readonly property color unbound: "#9e9e9e"

    readonly property int radius: 8
    readonly property int radiusSmall: 4
    readonly property int spacing: 8
    readonly property int spacingLarge: 16
    readonly property int fontSizeSmall: 11
    readonly property int fontSize: 13
    readonly property int fontSizeLarge: 16
    readonly property int fontSizeTitle: 20

    readonly property string fontFamily: "SF Pro Display, Segoe UI, Roboto, sans-serif"
    readonly property string fontMono: "JetBrains Mono, Fira Code, monospace"
}
